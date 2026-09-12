use alloc::vec::Vec;
use core::cell::UnsafeCell;
use super::ipv4::{self, Ipv4Address, Ipv4Packet, PROTO_UDP};

#[derive(Debug, Clone)]
pub struct UdpPacket {
    pub src_port: u16,
    pub dst_port: u16,
    pub length: u16,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct UdpDatagram {
    pub src_ip: Ipv4Address,
    pub src_port: u16,
    pub dst_port: u16,
    pub payload: Vec<u8>,
}

impl UdpPacket {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        let src_port = u16::from_be_bytes([data[0], data[1]]);
        let dst_port = u16::from_be_bytes([data[2], data[3]]);
        let length = u16::from_be_bytes([data[4], data[5]]);

        if (length as usize) < 8 || data.len() < (length as usize) {
            return None;
        }

        let payload = data[8..length as usize].to_vec();

        Some(Self {
            src_port,
            dst_port,
            length,
            payload,
        })
    }

    pub fn to_bytes(&self, src_ip: Ipv4Address, dst_ip: Ipv4Address) -> Vec<u8> {
        let length = (8 + self.payload.len()) as u16;
        let mut pseudo = Vec::with_capacity(12 + length as usize);

        pseudo.extend_from_slice(&src_ip);
        pseudo.extend_from_slice(&dst_ip);
        pseudo.push(0);
        pseudo.push(PROTO_UDP);
        pseudo.extend_from_slice(&length.to_be_bytes());

        let mut udp_hdr = Vec::with_capacity(length as usize);
        udp_hdr.extend_from_slice(&self.src_port.to_be_bytes());
        udp_hdr.extend_from_slice(&self.dst_port.to_be_bytes());
        udp_hdr.extend_from_slice(&length.to_be_bytes());
        udp_hdr.extend_from_slice(&[0, 0]); // Checksum placeholder
        udp_hdr.extend_from_slice(&self.payload);

        pseudo.extend_from_slice(&udp_hdr);
        let mut csum = ipv4::checksum(&pseudo);
        if csum == 0 {
            csum = 0xFFFF;
        }

        udp_hdr[6..8].copy_from_slice(&csum.to_be_bytes());
        udp_hdr
    }
}

const MAX_UDP_QUEUE: usize = 32;

struct SafeUdpQueue(UnsafeCell<Vec<UdpDatagram>>);
unsafe impl Sync for SafeUdpQueue {}

static UDP_QUEUE: SafeUdpQueue = SafeUdpQueue(UnsafeCell::new(Vec::new()));

pub fn handle_packet(ip_pkt: &Ipv4Packet) {
    if let Some(udp) = UdpPacket::parse(&ip_pkt.payload) {
        let q = unsafe { &mut *UDP_QUEUE.0.get() };
        if q.len() >= MAX_UDP_QUEUE {
            q.remove(0);
        }
        q.push(UdpDatagram {
            src_ip: ip_pkt.src,
            src_port: udp.src_port,
            dst_port: udp.dst_port,
            payload: udp.payload,
        });
    }
}

pub fn recv_from(port: u16) -> Option<UdpDatagram> {
    let q = unsafe { &mut *UDP_QUEUE.0.get() };
    for i in 0..q.len() {
        if q[i].dst_port == port {
            return Some(q.remove(i));
        }
    }
    None
}

pub fn clear_port(port: u16) {
    let q = unsafe { &mut *UDP_QUEUE.0.get() };
    q.retain(|pkt| pkt.dst_port != port);
}

pub fn send_to(
    dst_ip: Ipv4Address,
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
) -> Result<(), &'static str> {
    let cfg = match super::get_config() {
        Some(c) => c,
        None => return Err("Network uninitialized"),
    };

    let udp = UdpPacket {
        src_port,
        dst_port,
        length: (8 + payload.len()) as u16,
        payload: payload.to_vec(),
    };

    let bytes = udp.to_bytes(cfg.ip, dst_ip);
    super::ipv4_send(dst_ip, PROTO_UDP, bytes)
}
