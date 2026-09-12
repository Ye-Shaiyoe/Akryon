use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

pub type MacAddress = [u8; 6];

pub const ETHERTYPE_IPV4: u16 = 0x0800;
pub const ETHERTYPE_ARP: u16 = 0x0806;
pub const BROADCAST_MAC: MacAddress = [0xFF; 6];

#[derive(Debug, Clone)]
pub struct EthernetFrame {
    pub dest: MacAddress,
    pub src: MacAddress,
    pub ethertype: u16,
    pub payload: Vec<u8>,
}

impl EthernetFrame {
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 14 {
            return None;
        }

        let mut dest = [0u8; 6];
        let mut src = [0u8; 6];
        dest.copy_from_slice(&bytes[0..6]);
        src.copy_from_slice(&bytes[6..12]);
        let ethertype = u16::from_be_bytes([bytes[12], bytes[13]]);

        Some(Self {
            dest,
            src,
            ethertype,
            payload: bytes[14..].to_vec(),
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(14 + self.payload.len());
        out.extend_from_slice(&self.dest);
        out.extend_from_slice(&self.src);
        out.extend_from_slice(&self.ethertype.to_be_bytes());
        out.extend_from_slice(&self.payload);
        if out.len() < 60 {
            out.resize(60, 0);
        }
        out
    }
}

pub fn format_mac(mac: &MacAddress) -> String {
    format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5])
}
