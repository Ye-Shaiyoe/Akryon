use alloc::vec::Vec;
use core::cell::UnsafeCell;
use super::ipv4::{self, Ipv4Address, Ipv4Packet, PROTO_ICMP};

extern "C" {
    fn timer_get_uptime_ms() -> u32;
}

pub const ICMP_TYPE_ECHO_REPLY: u8 = 0;
pub const ICMP_TYPE_ECHO_REQUEST: u8 = 8;

#[derive(Debug, Clone)]
pub struct IcmpPacket {
    pub icmp_type: u8,
    pub code: u8,
    pub id: u16,
    pub seq: u16,
    pub payload: Vec<u8>,
}

impl IcmpPacket {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        let icmp_type = data[0];
        let code = data[1];
        let id = u16::from_be_bytes([data[4], data[5]]);
        let seq = u16::from_be_bytes([data[6], data[7]]);
        let payload = data[8..].to_vec();

        Some(Self {
            icmp_type,
            code,
            id,
            seq,
            payload,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + self.payload.len());
        out.push(self.icmp_type);
        out.push(self.code);
        out.extend_from_slice(&[0, 0]); // Checksum placeholder
        out.extend_from_slice(&self.id.to_be_bytes());
        out.extend_from_slice(&self.seq.to_be_bytes());
        out.extend_from_slice(&self.payload);

        let csum = ipv4::checksum(&out);
        out[2..4].copy_from_slice(&csum.to_be_bytes());
        out
    }
}

struct SafePingState(UnsafeCell<Option<(u16, u16, u32)>>);
unsafe impl Sync for SafePingState {}

static PING_STATE: SafePingState = SafePingState(UnsafeCell::new(None));

pub fn handle_packet(ip_pkt: &Ipv4Packet, my_ip: &Ipv4Address) {
    let icmp = match IcmpPacket::parse(&ip_pkt.payload) {
        Some(pkt) => pkt,
        None => return,
    };

    if icmp.icmp_type == ICMP_TYPE_ECHO_REQUEST && &ip_pkt.dst == my_ip {
        let reply = IcmpPacket {
            icmp_type: ICMP_TYPE_ECHO_REPLY,
            code: 0,
            id: icmp.id,
            seq: icmp.seq,
            payload: icmp.payload,
        };

        let _ = super::ipv4_send(ip_pkt.src, PROTO_ICMP, reply.to_bytes());
    } else if icmp.icmp_type == ICMP_TYPE_ECHO_REPLY {
        let now = unsafe { timer_get_uptime_ms() };
        unsafe {
            *PING_STATE.0.get() = Some((icmp.id, icmp.seq, now));
        }
    }
}

pub fn send_ping(target_ip: Ipv4Address, seq: u16, timeout_ms: u32) -> Result<u32, &'static str> {
    let ping_id = 0x414Bu16;
    let req = IcmpPacket {
        icmp_type: ICMP_TYPE_ECHO_REQUEST,
        code: 0,
        id: ping_id,
        seq,
        payload: b"AkryonPingPacket".to_vec(),
    };

    unsafe {
        *PING_STATE.0.get() = None;
    }

    let start = unsafe { timer_get_uptime_ms() };
    if super::ipv4_send(target_ip, PROTO_ICMP, req.to_bytes()).is_err() {
        return Err("Failed to transmit ICMP packet");
    }

    while (unsafe { timer_get_uptime_ms() } - start) < timeout_ms {
        super::poll();
        unsafe {
            if let Some((id, rep_seq, rec_time)) = *PING_STATE.0.get() {
                if id == ping_id && rep_seq == seq {
                    *PING_STATE.0.get() = None;
                    return Ok(rec_time.saturating_sub(start));
                }
            }
        }
    }

    Err("Request timeout")
}
