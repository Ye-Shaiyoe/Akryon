use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use core::cell::UnsafeCell;
use crate::logln;

extern "C" {
    fn rtl8139_is_active() -> i32;
    fn rtl8139_get_mac(mac_out: *mut u8) -> i32;
    fn rtl8139_send_packet(data: *const u8, len: u32) -> i32;
    fn rtl8139_receive_packet(buf: *mut u8, max_len: u32) -> i32;
    fn rtl8139_get_stats(rx_pkts: *mut u32, tx_pkts: *mut u32, rx_bytes: *mut u32, tx_bytes: *mut u32);
    fn timer_get_uptime_ms() -> u32;
}

#[derive(Debug, Clone, Copy)]
pub struct NetConfig {
    pub mac: [u8; 6],
    pub ip: [u8; 4],
    pub netmask: [u8; 4],
    pub gateway: [u8; 4],
    pub dns: [u8; 4],
    pub is_up: bool,
}

struct SafeNet(UnsafeCell<Option<NetConfig>>);
unsafe impl Sync for SafeNet {}

static NET_STATE: SafeNet = SafeNet(UnsafeCell::new(None));

pub fn is_card_active() -> bool {
    unsafe { rtl8139_is_active() != 0 }
}

pub fn get_config() -> Option<NetConfig> {
    unsafe { *NET_STATE.0.get() }
}

pub fn get_stats() -> (u32, u32, u32, u32) {
    let mut rx_pkts = 0u32;
    let mut tx_pkts = 0u32;
    let mut rx_bytes = 0u32;
    let mut tx_bytes = 0u32;
    unsafe {
        rtl8139_get_stats(&mut rx_pkts, &mut tx_pkts, &mut rx_bytes, &mut tx_bytes);
    }
    (rx_pkts, tx_pkts, rx_bytes, tx_bytes)
}

pub fn format_ip(ip: &[u8; 4]) -> String {
    format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])
}

pub fn format_mac(mac: &[u8; 6]) -> String {
    format!("{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5])
}

pub fn parse_ip(s: &str) -> Option<[u8; 4]> {
    let parts: Vec<&str> = s.trim().split('.').collect();
    if parts.len() != 4 {
        return None;
    }

    let mut ip = [0u8; 4];
    for i in 0..4 {
        match parts[i].parse::<u8>() {
            Ok(val) => ip[i] = val,
            Err(_) => return None,
        }
    }
    Some(ip)
}

fn calculate_checksum(data: &[u8]) -> u16 {
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

pub fn update_config(ip: [u8; 4], netmask: [u8; 4], gateway: [u8; 4]) {
    unsafe {
        if let Some(cfg) = (&mut *NET_STATE.0.get()).as_mut() {
            cfg.ip = ip;
            cfg.netmask = netmask;
            cfg.gateway = gateway;
            save_config_to_vfs(cfg);
        }
    }
}

fn save_config_to_vfs(cfg: &NetConfig) {
    let content = format!(
        "INTERFACE=eth0\nIP={}\nNETMASK={}\nGATEWAY={}\nDNS={}\nMAC={}\n",
        format_ip(&cfg.ip),
        format_ip(&cfg.netmask),
        format_ip(&cfg.gateway),
        format_ip(&cfg.dns),
        format_mac(&cfg.mac)
    );
    let _ = crate::vfs::write_file("/etc/network.conf", content.as_bytes());

    let resolv = format!("nameserver {}\n", format_ip(&cfg.dns));
    let _ = crate::vfs::write_file("/etc/resolv.conf", resolv.as_bytes());
}

pub fn init() {
    let mut mac = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
    let active = is_card_active();

    if active {
        unsafe {
            rtl8139_get_mac(mac.as_mut_ptr());
        }
    }

    let mut cfg = NetConfig {
        mac,
        ip: [10, 0, 2, 15],
        netmask: [255, 255, 255, 0],
        gateway: [10, 0, 2, 2],
        dns: [10, 0, 2, 3],
        is_up: active,
    };

    if let Some(data) = crate::vfs::read_file("/etc/network.conf") {
        if let Ok(text) = core::str::from_utf8(&data) {
            for line in text.lines() {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim();
                    let val = parts[1].trim();
                    match key {
                        "IP" => if let Some(ip) = parse_ip(val) { cfg.ip = ip; },
                        "NETMASK" => if let Some(m) = parse_ip(val) { cfg.netmask = m; },
                        "GATEWAY" => if let Some(gw) = parse_ip(val) { cfg.gateway = gw; },
                        "DNS" => if let Some(dns) = parse_ip(val) { cfg.dns = dns; },
                        _ => {}
                    }
                }
            }
        }
    } else {
        save_config_to_vfs(&cfg);
    }

    logln!("[Akryon Net] Initialized interface eth0 (MAC: {}, IP: {})",
        format_mac(&cfg.mac), format_ip(&cfg.ip));

    unsafe {
        *NET_STATE.0.get() = Some(cfg);
    }
}

static mut ARP_CACHE_IP: [u8; 4] = [0; 4];
static mut ARP_CACHE_MAC: [u8; 6] = [0; 6];
static mut ARP_CACHE_VALID: bool = false;

pub fn arp_resolve(target_ip: [u8; 4]) -> Result<[u8; 6], &'static str> {
    unsafe {
        if ARP_CACHE_VALID && ARP_CACHE_IP == target_ip {
            return Ok(ARP_CACHE_MAC);
        }
    }

    let cfg = match get_config() {
        Some(c) => c,
        None => return Err("Network subsystem uninitialized"),
    };

    let mut arp_frame = Vec::with_capacity(60);
    arp_frame.extend_from_slice(&[0xFF; 6]);
    arp_frame.extend_from_slice(&cfg.mac);
    arp_frame.extend_from_slice(&0x0806u16.to_be_bytes()); // EtherType: ARP

    arp_frame.extend_from_slice(&0x0001u16.to_be_bytes()); // HW Type: Ethernet (1)
    arp_frame.extend_from_slice(&0x0800u16.to_be_bytes()); // Proto Type: IPv4 (0x0800)
    arp_frame.push(6);                                    // HW Size: 6
    arp_frame.push(4);                                    // Proto Size: 4
    arp_frame.extend_from_slice(&0x0001u16.to_be_bytes()); // Opcode: Request (1)
    arp_frame.extend_from_slice(&cfg.mac);                 // Sender MAC
    arp_frame.extend_from_slice(&cfg.ip);                  // Sender IP
    arp_frame.extend_from_slice(&[0x00; 6]);               // Target MAC
    arp_frame.extend_from_slice(&target_ip);               // Target IP

    if arp_frame.len() < 60 {
        arp_frame.resize(60, 0);
    }

    let start = unsafe { timer_get_uptime_ms() };
    let ret = unsafe { rtl8139_send_packet(arp_frame.as_ptr(), arp_frame.len() as u32) };
    if ret < 0 {
        return Err("Failed to send ARP frame");
    }

    let mut rx_buf = [0u8; 1536];
    let timeout = 1000u32;
    while (unsafe { timer_get_uptime_ms() } - start) < timeout {
        let n = unsafe { rtl8139_receive_packet(rx_buf.as_mut_ptr(), 1536) };
        if n >= 42 {
            let eth_type = u16::from_be_bytes([rx_buf[12], rx_buf[13]]);
            if eth_type == 0x0806 {
                let opcode = u16::from_be_bytes([rx_buf[20], rx_buf[21]]);
                if opcode == 2 && &rx_buf[28..32] == &target_ip {
                    let mut resolved = [0u8; 6];
                    resolved.copy_from_slice(&rx_buf[22..28]);
                    unsafe {
                        ARP_CACHE_IP = target_ip;
                        ARP_CACHE_MAC = resolved;
                        ARP_CACHE_VALID = true;
                    }
                    return Ok(resolved);
                }
            }
        }
    }

    Err("ARP resolution timed out")
}

pub fn send_ping(target_ip: [u8; 4], seq: u16) -> Result<u32, &'static str> {
    if !is_card_active() {
        return Err("Network card RTL8139 not detected or inactive");
    }

    let cfg = match get_config() {
        Some(c) => c,
        None => return Err("Network subsystem uninitialized"),
    };

    let dest_mac = match arp_resolve(target_ip) {
        Ok(m) => m,
        Err(_) => [0xFF; 6],
    };

    let icmp_len = 8 + 16;
    let mut icmp_packet = Vec::with_capacity(icmp_len);
    icmp_packet.push(8); // Type 8: Echo Request
    icmp_packet.push(0); // Code 0
    icmp_packet.extend_from_slice(&[0, 0]); // Checksum placeholder
    icmp_packet.extend_from_slice(&0x1234u16.to_be_bytes()); // ID
    icmp_packet.extend_from_slice(&seq.to_be_bytes()); // Sequence number
    icmp_packet.extend_from_slice(b"AkryonPingPacket"); // Payload

    let icmp_csum = calculate_checksum(&icmp_packet);
    icmp_packet[2..4].copy_from_slice(&icmp_csum.to_be_bytes());

    let ip_total_len = (20 + icmp_packet.len()) as u16;
    let mut ip_packet = Vec::with_capacity(20 + icmp_packet.len());
    ip_packet.push(0x45); // IPv4, 20 bytes
    ip_packet.push(0x00); // DSCP / ECN
    ip_packet.extend_from_slice(&ip_total_len.to_be_bytes());
    ip_packet.extend_from_slice(&seq.to_be_bytes()); // ID
    ip_packet.extend_from_slice(&0x4000u16.to_be_bytes()); // Flags: Don't Fragment
    ip_packet.push(64);   // TTL
    ip_packet.push(1);    // Protocol: ICMP
    ip_packet.extend_from_slice(&[0, 0]); // Checksum placeholder
    ip_packet.extend_from_slice(&cfg.ip);
    ip_packet.extend_from_slice(&target_ip);

    let ip_csum = calculate_checksum(&ip_packet[..20]);
    ip_packet[10..12].copy_from_slice(&ip_csum.to_be_bytes());
    ip_packet.extend_from_slice(&icmp_packet);

    let mut eth_frame = Vec::with_capacity(14 + ip_packet.len());
    eth_frame.extend_from_slice(&dest_mac);
    eth_frame.extend_from_slice(&cfg.mac);
    eth_frame.extend_from_slice(&0x0800u16.to_be_bytes()); // EtherType: IPv4
    eth_frame.extend_from_slice(&ip_packet);

    if eth_frame.len() < 60 {
        eth_frame.resize(60, 0); // Ethernet minimum frame length padding
    }

    let start_time = unsafe { timer_get_uptime_ms() };
    let ret = unsafe { rtl8139_send_packet(eth_frame.as_ptr(), eth_frame.len() as u32) };
    if ret < 0 {
        return Err("Failed to send frame via RTL8139");
    }

    let mut rx_buf = [0u8; 1536];
    let timeout_ms = 1500u32;

    while (unsafe { timer_get_uptime_ms() } - start_time) < timeout_ms {
        let n = unsafe { rtl8139_receive_packet(rx_buf.as_mut_ptr(), 1536) };
        if n >= 34 {
            let eth_type = u16::from_be_bytes([rx_buf[12], rx_buf[13]]);
            if eth_type == 0x0800 {
                let ip_hdr = &rx_buf[14..];
                let proto = ip_hdr[9];
                if proto == 1 { // ICMP
                    let icmp = &ip_hdr[20..];
                    let icmp_type = icmp[0];
                    if icmp_type == 0 { // Echo Reply
                        let elapsed = unsafe { timer_get_uptime_ms() } - start_time;
                        return Ok(elapsed);
                    }
                }
            }
        }
    }

    Err("Request timeout (no reply received within 1500ms)")
}
