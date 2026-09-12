use alloc::vec::Vec;
use core::cell::UnsafeCell;
use super::ethernet::{EthernetFrame, MacAddress, BROADCAST_MAC, ETHERTYPE_ARP};

extern "C" {
    fn timer_get_uptime_ms() -> u32;
}

pub const ARP_OP_REQUEST: u16 = 1;
pub const ARP_OP_REPLY: u16 = 2;

#[derive(Debug, Clone, Copy)]
pub struct ArpPacket {
    pub hw_type: u16,
    pub proto_type: u16,
    pub hw_len: u8,
    pub proto_len: u8,
    pub opcode: u16,
    pub sender_mac: MacAddress,
    pub sender_ip: [u8; 4],
    pub target_mac: MacAddress,
    pub target_ip: [u8; 4],
}

impl ArpPacket {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 28 {
            return None;
        }

        let hw_type = u16::from_be_bytes([data[0], data[1]]);
        let proto_type = u16::from_be_bytes([data[2], data[3]]);
        let hw_len = data[4];
        let proto_len = data[5];
        let opcode = u16::from_be_bytes([data[6], data[7]]);

        if hw_type != 1 || proto_type != 0x0800 || hw_len != 6 || proto_len != 4 {
            return None;
        }

        let mut sender_mac = [0u8; 6];
        let mut sender_ip = [0u8; 4];
        let mut target_mac = [0u8; 6];
        let mut target_ip = [0u8; 4];

        sender_mac.copy_from_slice(&data[8..14]);
        sender_ip.copy_from_slice(&data[14..18]);
        target_mac.copy_from_slice(&data[18..24]);
        target_ip.copy_from_slice(&data[24..28]);

        Some(Self {
            hw_type,
            proto_type,
            hw_len,
            proto_len,
            opcode,
            sender_mac,
            sender_ip,
            target_mac,
            target_ip,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(28);
        out.extend_from_slice(&self.hw_type.to_be_bytes());
        out.extend_from_slice(&self.proto_type.to_be_bytes());
        out.push(self.hw_len);
        out.push(self.proto_len);
        out.extend_from_slice(&self.opcode.to_be_bytes());
        out.extend_from_slice(&self.sender_mac);
        out.extend_from_slice(&self.sender_ip);
        out.extend_from_slice(&self.target_mac);
        out.extend_from_slice(&self.target_ip);
        out
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ArpEntry {
    pub ip: [u8; 4],
    pub mac: MacAddress,
    pub updated_ms: u32,
}

const MAX_ARP_ENTRIES: usize = 32;

struct SafeArpTable(UnsafeCell<Vec<ArpEntry>>);
unsafe impl Sync for SafeArpTable {}

static ARP_TABLE: SafeArpTable = SafeArpTable(UnsafeCell::new(Vec::new()));

pub fn update_cache(ip: [u8; 4], mac: MacAddress) {
    if ip == [0, 0, 0, 0] || mac == [0; 6] {
        return;
    }

    let now = unsafe { timer_get_uptime_ms() };
    let table = unsafe { &mut *ARP_TABLE.0.get() };

    for entry in table.iter_mut() {
        if entry.ip == ip {
            entry.mac = mac;
            entry.updated_ms = now;
            return;
        }
    }

    if table.len() >= MAX_ARP_ENTRIES {
        table.remove(0);
    }

    table.push(ArpEntry {
        ip,
        mac,
        updated_ms: now,
    });
}

pub fn lookup_cache(ip: &[u8; 4]) -> Option<MacAddress> {
    let table = unsafe { &*ARP_TABLE.0.get() };
    for entry in table.iter() {
        if &entry.ip == ip {
            return Some(entry.mac);
        }
    }
    None
}

pub fn get_table() -> Vec<ArpEntry> {
    let table = unsafe { &*ARP_TABLE.0.get() };
    table.clone()
}

pub fn flush_cache() {
    let table = unsafe { &mut *ARP_TABLE.0.get() };
    table.clear();
}

pub fn handle_packet(arp: &ArpPacket, my_mac: &MacAddress, my_ip: &[u8; 4]) -> Option<Vec<u8>> {
    update_cache(arp.sender_ip, arp.sender_mac);

    if arp.opcode == ARP_OP_REQUEST && arp.target_ip == *my_ip {
        let reply = ArpPacket {
            hw_type: 1,
            proto_type: 0x0800,
            hw_len: 6,
            proto_len: 4,
            opcode: ARP_OP_REPLY,
            sender_mac: *my_mac,
            sender_ip: *my_ip,
            target_mac: arp.sender_mac,
            target_ip: arp.sender_ip,
        };

        let frame = EthernetFrame {
            dest: arp.sender_mac,
            src: *my_mac,
            ethertype: ETHERTYPE_ARP,
            payload: reply.to_bytes(),
        };

        return Some(frame.to_bytes());
    }

    None
}

pub fn resolve(target_ip: [u8; 4], timeout_ms: u32) -> Result<MacAddress, &'static str> {
    if let Some(mac) = lookup_cache(&target_ip) {
        return Ok(mac);
    }

    let cfg = match super::get_config() {
        Some(c) => c,
        None => return Err("Network uninitialized"),
    };

    let req = ArpPacket {
        hw_type: 1,
        proto_type: 0x0800,
        hw_len: 6,
        proto_len: 4,
        opcode: ARP_OP_REQUEST,
        sender_mac: cfg.mac,
        sender_ip: cfg.ip,
        target_mac: [0; 6],
        target_ip,
    };

    let frame = EthernetFrame {
        dest: BROADCAST_MAC,
        src: cfg.mac,
        ethertype: ETHERTYPE_ARP,
        payload: req.to_bytes(),
    };

    if super::send_raw(&frame.to_bytes()).is_err() {
        return Err("Failed to send ARP request");
    }

    let start = unsafe { timer_get_uptime_ms() };
    while (unsafe { timer_get_uptime_ms() } - start) < timeout_ms {
        super::poll();
        if let Some(mac) = lookup_cache(&target_ip) {
            return Ok(mac);
        }
    }

    Err("ARP resolution timed out")
}
