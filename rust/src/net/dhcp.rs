use alloc::vec::Vec;
use super::ipv4::Ipv4Address;
use super::ethernet::MacAddress;
use super::udp;

extern "C" {
    fn timer_get_uptime_ms() -> u32;
}

pub const DHCP_SERVER_PORT: u16 = 67;
pub const DHCP_CLIENT_PORT: u16 = 68;

const MAGIC_COOKIE: [u8; 4] = [99, 130, 83, 99];

#[derive(Debug, Clone)]
pub struct DhcpLease {
    pub ip: Ipv4Address,
    pub netmask: Ipv4Address,
    pub gateway: Ipv4Address,
    pub dns: Ipv4Address,
    pub lease_sec: u32,
}

fn build_dhcp_base(op: u8, xid: u32, mac: &MacAddress, options: &[u8]) -> Vec<u8> {
    let mut pkt = Vec::with_capacity(240 + options.len() + 1);
    pkt.push(op); // 1 = BootRequest
    pkt.push(1);  // Hardware type: Ethernet
    pkt.push(6);  // Hardware address length
    pkt.push(0);  // Hops
    pkt.extend_from_slice(&xid.to_be_bytes());
    pkt.extend_from_slice(&0u16.to_be_bytes()); // Secs
    pkt.extend_from_slice(&0x8000u16.to_be_bytes()); // Flags: Broadcast
    pkt.extend_from_slice(&[0; 4]); // ciaddr
    pkt.extend_from_slice(&[0; 4]); // yiaddr
    pkt.extend_from_slice(&[0; 4]); // siaddr
    pkt.extend_from_slice(&[0; 4]); // giaddr

    // chaddr: MAC + 10 zero bytes
    pkt.extend_from_slice(mac);
    pkt.extend_from_slice(&[0; 10]);

    // sname: 64 bytes zero
    pkt.extend_from_slice(&[0; 64]);
    // file: 128 bytes zero
    pkt.extend_from_slice(&[0; 128]);

    // Magic cookie
    pkt.extend_from_slice(&MAGIC_COOKIE);
    pkt.extend_from_slice(options);
    pkt.push(0xFF); // End option
    pkt
}

fn find_option<'a>(data: &'a [u8], opt_code: u8) -> Option<&'a [u8]> {
    if data.len() < 240 {
        return None;
    }
    if &data[236..240] != &MAGIC_COOKIE {
        return None;
    }

    let mut i = 240;
    while i < data.len() {
        let code = data[i];
        if code == 0xFF {
            break;
        }
        if code == 0x00 {
            i += 1;
            continue;
        }
        if i + 1 >= data.len() {
            break;
        }
        let len = data[i + 1] as usize;
        let end = i + 2 + len;
        if end > data.len() {
            break;
        }
        if code == opt_code {
            return Some(&data[i + 2..end]);
        }
        i = end;
    }
    None
}

pub fn request_lease() -> Result<DhcpLease, &'static str> {
    let cfg = match super::get_config() {
        Some(c) => c,
        None => return Err("Network uninitialized"),
    };

    udp::clear_port(DHCP_CLIENT_PORT);
    let xid = 0x5348_5031u32; // Deterministic XID

    // 1. DHCP Discover
    let mut disc_opts = Vec::new();
    disc_opts.extend_from_slice(&[53, 1, 1]); // Option 53: Discover (1)
    disc_opts.extend_from_slice(&[55, 4, 1, 3, 6, 51]); // Param request: Mask, Router, DNS, Lease

    let disc_pkt = build_dhcp_base(1, xid, &cfg.mac, &disc_opts);
    let bcast_ip = [255, 255, 255, 255];

    // Send Discover via UDP broadcast
    let _ = udp::send_to(bcast_ip, DHCP_CLIENT_PORT, DHCP_SERVER_PORT, &disc_pkt);

    // 2. Wait for DHCP Offer
    let start = unsafe { timer_get_uptime_ms() };
    let timeout = 3000u32;
    let mut offered_ip = [0u8; 4];
    let mut server_id = [0u8; 4];

    let mut offer_found = false;
    while (unsafe { timer_get_uptime_ms() } - start) < timeout {
        super::poll();
        if let Some(dgram) = udp::recv_from(DHCP_CLIENT_PORT) {
            if dgram.payload.len() >= 240 {
                let rx_xid = u32::from_be_bytes([
                    dgram.payload[4], dgram.payload[5], dgram.payload[6], dgram.payload[7]
                ]);
                if rx_xid == xid {
                    if let Some(msg_type) = find_option(&dgram.payload, 53) {
                        if msg_type.len() == 1 && msg_type[0] == 2 { // 2 = Offer
                            offered_ip.copy_from_slice(&dgram.payload[16..20]);
                            if let Some(sid) = find_option(&dgram.payload, 54) {
                                if sid.len() >= 4 {
                                    server_id.copy_from_slice(&sid[..4]);
                                }
                            } else {
                                server_id = dgram.src_ip;
                            }
                            offer_found = true;
                            break;
                        }
                    }
                }
            }
        }
    }

    if !offer_found {
        return Err("DHCP Discover timed out (no DHCP Offer received)");
    }

    // 3. DHCP Request
    let mut req_opts = Vec::new();
    req_opts.extend_from_slice(&[53, 1, 3]); // Option 53: Request (3)
    req_opts.extend_from_slice(&[50, 4, offered_ip[0], offered_ip[1], offered_ip[2], offered_ip[3]]);
    req_opts.extend_from_slice(&[54, 4, server_id[0], server_id[1], server_id[2], server_id[3]]);
    req_opts.extend_from_slice(&[55, 4, 1, 3, 6, 51]);

    let req_pkt = build_dhcp_base(1, xid, &cfg.mac, &req_opts);
    let _ = udp::send_to(bcast_ip, DHCP_CLIENT_PORT, DHCP_SERVER_PORT, &req_pkt);

    // 4. Wait for DHCP Ack
    let ack_start = unsafe { timer_get_uptime_ms() };
    while (unsafe { timer_get_uptime_ms() } - ack_start) < timeout {
        super::poll();
        if let Some(dgram) = udp::recv_from(DHCP_CLIENT_PORT) {
            if dgram.payload.len() >= 240 {
                let rx_xid = u32::from_be_bytes([
                    dgram.payload[4], dgram.payload[5], dgram.payload[6], dgram.payload[7]
                ]);
                if rx_xid == xid {
                    if let Some(msg_type) = find_option(&dgram.payload, 53) {
                        if msg_type.len() == 1 && msg_type[0] == 5 { // 5 = Ack
                            let mut netmask = [255, 255, 255, 0];
                            let mut gateway = [10, 0, 2, 2];
                            let mut dns = [10, 0, 2, 3];
                            let mut lease_sec = 86400u32;

                            if let Some(m) = find_option(&dgram.payload, 1) {
                                if m.len() >= 4 { netmask.copy_from_slice(&m[..4]); }
                            }
                            if let Some(gw) = find_option(&dgram.payload, 3) {
                                if gw.len() >= 4 { gateway.copy_from_slice(&gw[..4]); }
                            }
                            if let Some(ns) = find_option(&dgram.payload, 6) {
                                if ns.len() >= 4 { dns.copy_from_slice(&ns[..4]); }
                            }
                            if let Some(ls) = find_option(&dgram.payload, 51) {
                                if ls.len() >= 4 {
                                    lease_sec = u32::from_be_bytes([ls[0], ls[1], ls[2], ls[3]]);
                                }
                            }

                            let lease = DhcpLease {
                                ip: offered_ip,
                                netmask,
                                gateway,
                                dns,
                                lease_sec,
                            };

                            super::update_config_all(offered_ip, netmask, gateway, dns);
                            return Ok(lease);
                        }
                    }
                }
            }
        }
    }

    Err("DHCP Request timed out (no DHCP Ack received)")
}
