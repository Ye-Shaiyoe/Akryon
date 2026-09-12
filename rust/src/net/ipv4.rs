use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

pub type Ipv4Address = [u8; 4];

pub const PROTO_ICMP: u8 = 1;
pub const PROTO_TCP: u8 = 6;
pub const PROTO_UDP: u8 = 17;

#[derive(Debug, Clone)]
pub struct Ipv4Packet {
    pub tos: u8,
    pub id: u16,
    pub flags_frag: u16,
    pub ttl: u8,
    pub proto: u8,
    pub src: Ipv4Address,
    pub dst: Ipv4Address,
    pub payload: Vec<u8>,
}

pub fn checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let mut i = 0;
    while i + 1 < data.len() {
        let word = u16::from_be_bytes([data[i], data[i + 1]]);
        sum += word as u32;
        i += 2;
    }
    if i < data.len() {
        sum += (data[i] as u32) << 8;
    }
    while (sum >> 16) > 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !sum as u16
}

impl Ipv4Packet {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 20 {
            return None;
        }

        let version = data[0] >> 4;
        let ihl = (data[0] & 0x0F) as usize * 4;
        if version != 4 || ihl < 20 || data.len() < ihl {
            return None;
        }

        let total_len = u16::from_be_bytes([data[2], data[3]]) as usize;
        if total_len < ihl || data.len() < total_len {
            return None;
        }

        if checksum(&data[..ihl]) != 0 {
            return None;
        }

        let tos = data[1];
        let id = u16::from_be_bytes([data[4], data[5]]);
        let flags_frag = u16::from_be_bytes([data[6], data[7]]);
        let ttl = data[8];
        let proto = data[9];

        let mut src = [0u8; 4];
        let mut dst = [0u8; 4];
        src.copy_from_slice(&data[12..16]);
        dst.copy_from_slice(&data[16..20]);

        let payload = data[ihl..total_len].to_vec();

        Some(Self {
            tos,
            id,
            flags_frag,
            ttl,
            proto,
            src,
            dst,
            payload,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let total_len = (20 + self.payload.len()) as u16;
        let mut out = Vec::with_capacity(total_len as usize);

        out.push(0x45); // IPv4, IHL=5 (20 bytes)
        out.push(self.tos);
        out.extend_from_slice(&total_len.to_be_bytes());
        out.extend_from_slice(&self.id.to_be_bytes());
        out.extend_from_slice(&self.flags_frag.to_be_bytes());
        out.push(self.ttl);
        out.push(self.proto);
        out.extend_from_slice(&[0, 0]); // Checksum placeholder
        out.extend_from_slice(&self.src);
        out.extend_from_slice(&self.dst);

        let csum = checksum(&out[..20]);
        out[10..12].copy_from_slice(&csum.to_be_bytes());
        out.extend_from_slice(&self.payload);
        out
    }
}

pub fn route_destination(
    dst: Ipv4Address,
    my_ip: Ipv4Address,
    mask: Ipv4Address,
    gw: Ipv4Address,
) -> Ipv4Address {
    if dst == [255, 255, 255, 255] || dst == [0, 0, 0, 0] {
        return dst;
    }

    let is_local = (dst[0] & mask[0] == my_ip[0] & mask[0])
        && (dst[1] & mask[1] == my_ip[1] & mask[1])
        && (dst[2] & mask[2] == my_ip[2] & mask[2])
        && (dst[3] & mask[3] == my_ip[3] & mask[3]);

    if is_local {
        dst
    } else {
        gw
    }
}

pub fn format_ip(ip: &Ipv4Address) -> String {
    format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])
}

pub fn parse_ip(s: &str) -> Option<Ipv4Address> {
    let mut parts = s.trim().split('.');
    let mut ip = [0u8; 4];
    for b in &mut ip {
        *b = parts.next()?.parse::<u8>().ok()?;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(ip)
}
