use alloc::vec::Vec;
use super::ipv4::{self, Ipv4Address};
use super::udp;

extern "C" {
    fn timer_get_uptime_ms() -> u32;
}

pub const DNS_PORT: u16 = 53;
const DNS_CLIENT_PORT: u16 = 53421;

fn encode_dns_name(hostname: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    for label in hostname.split('.') {
        if label.is_empty() || label.len() > 63 {
            return None;
        }
        out.push(label.len() as u8);
        out.extend_from_slice(label.as_bytes());
    }
    out.push(0);
    Some(out)
}

fn skip_dns_name(data: &[u8], mut offset: usize) -> Option<usize> {
    while offset < data.len() {
        let len = data[offset];
        if len == 0 {
            return Some(offset + 1);
        }
        if (len & 0xC0) == 0xC0 {
            return Some(offset + 2);
        }
        offset += 1 + (len as usize);
    }
    None
}

pub fn resolve(hostname: &str, timeout_ms: u32) -> Result<Ipv4Address, &'static str> {
    if let Some(ip) = ipv4::parse_ip(hostname) {
        return Ok(ip);
    }

    let cfg = match super::get_config() {
        Some(c) => c,
        None => return Err("Network uninitialized"),
    };

    let qname = match encode_dns_name(hostname.trim()) {
        Some(q) => q,
        None => return Err("Invalid domain name"),
    };

    let tx_id = 0x414Bu16;
    let mut query = Vec::with_capacity(12 + qname.len() + 4);
    query.extend_from_slice(&tx_id.to_be_bytes());
    query.extend_from_slice(&0x0100u16.to_be_bytes()); // Flags: Standard query, Recursion Desired
    query.extend_from_slice(&1u16.to_be_bytes());      // Questions: 1
    query.extend_from_slice(&0u16.to_be_bytes());      // Answers: 0
    query.extend_from_slice(&0u16.to_be_bytes());      // Authority: 0
    query.extend_from_slice(&0u16.to_be_bytes());      // Additional: 0
    query.extend_from_slice(&qname);
    query.extend_from_slice(&1u16.to_be_bytes());      // Type: A (Host Address)
    query.extend_from_slice(&1u16.to_be_bytes());      // Class: IN (Internet)

    udp::clear_port(DNS_CLIENT_PORT);
    let _ = udp::send_to(cfg.dns, DNS_CLIENT_PORT, DNS_PORT, &query);

    let start = unsafe { timer_get_uptime_ms() };
    while (unsafe { timer_get_uptime_ms() } - start) < timeout_ms {
        super::poll();
        if let Some(dgram) = udp::recv_from(DNS_CLIENT_PORT) {
            let data = &dgram.payload;
            if data.len() < 12 {
                continue;
            }

            let rx_id = u16::from_be_bytes([data[0], data[1]]);
            if rx_id != tx_id {
                continue;
            }

            let flags = u16::from_be_bytes([data[2], data[3]]);
            let rcode = flags & 0x000F;
            if rcode != 0 {
                return Err("DNS query rejected or domain not found");
            }

            let qdcount = u16::from_be_bytes([data[4], data[5]]) as usize;
            let ancount = u16::from_be_bytes([data[6], data[7]]) as usize;

            if ancount == 0 {
                return Err("Domain resolved but no A records found");
            }

            let mut offset = 12;
            // Skip questions
            for _ in 0..qdcount {
                offset = match skip_dns_name(data, offset) {
                    Some(off) => off + 4, // skip QTYPE + QCLASS
                    None => return Err("Malformed DNS response"),
                };
            }

            // Parse answers
            for _ in 0..ancount {
                offset = match skip_dns_name(data, offset) {
                    Some(off) => off,
                    None => return Err("Malformed DNS answer name"),
                };

                if offset + 10 > data.len() {
                    return Err("Truncated DNS record");
                }

                let atype = u16::from_be_bytes([data[offset], data[offset + 1]]);
                let rdlength = u16::from_be_bytes([data[offset + 8], data[offset + 9]]) as usize;
                offset += 10;

                if atype == 1 && rdlength == 4 { // Type A
                    if offset + 4 <= data.len() {
                        let mut ip = [0u8; 4];
                        ip.copy_from_slice(&data[offset..offset + 4]);
                        return Ok(ip);
                    }
                }

                offset += rdlength;
            }

            return Err("No IPv4 address in DNS answer");
        }
    }

    Err("DNS resolution timed out")
}
