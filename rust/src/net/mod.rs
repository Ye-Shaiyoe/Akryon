use alloc::vec::Vec;
use alloc::format;
use core::cell::UnsafeCell;
use crate::logln;

pub mod ethernet;
pub mod arp;
pub mod ipv4;
pub mod icmp;
pub mod udp;
pub mod dhcp;
pub mod dns;
pub mod tcp;
pub mod http;

pub use self::ethernet::{format_mac, MacAddress, BROADCAST_MAC, ETHERTYPE_ARP, ETHERTYPE_IPV4};
pub use self::ipv4::{format_ip, parse_ip, Ipv4Address, PROTO_ICMP, PROTO_TCP, PROTO_UDP};
pub use self::icmp::send_ping;
pub use self::dhcp::{request_lease, DhcpLease};
pub use self::dns::resolve as dns_resolve;
pub use self::http::{fetch, handle_http_request};

extern "C" {
    pub fn rtl8139_is_active() -> i32;
    pub fn rtl8139_has_packet() -> i32;
    pub fn rtl8139_get_mac(mac_out: *mut u8) -> i32;
    pub fn rtl8139_send_packet(data: *const u8, len: u32) -> i32;
    pub fn rtl8139_receive_packet(buf: *mut u8, max_len: u32) -> i32;
    pub fn rtl8139_get_stats(rx_pkts: *mut u32, tx_pkts: *mut u32, rx_bytes: *mut u32, tx_bytes: *mut u32);
    pub fn timer_get_uptime_ms() -> u32;
}

#[derive(Debug, Clone, Copy)]
pub struct NetConfig {
    pub mac: MacAddress,
    pub ip: Ipv4Address,
    pub netmask: Ipv4Address,
    pub gateway: Ipv4Address,
    pub dns: Ipv4Address,
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

pub fn update_config(ip: Ipv4Address, netmask: Ipv4Address, gateway: Ipv4Address) {
    unsafe {
        if let Some(cfg) = (&mut *NET_STATE.0.get()).as_mut() {
            cfg.ip = ip;
            cfg.netmask = netmask;
            cfg.gateway = gateway;
            save_config_to_vfs(cfg);
        }
    }
}

pub fn update_config_all(ip: Ipv4Address, netmask: Ipv4Address, gateway: Ipv4Address, dns: Ipv4Address) {
    unsafe {
        if let Some(cfg) = (&mut *NET_STATE.0.get()).as_mut() {
            cfg.ip = ip;
            cfg.netmask = netmask;
            cfg.gateway = gateway;
            cfg.dns = dns;
            save_config_to_vfs(cfg);
        }
    }
}

pub fn save_config_to_vfs(cfg: &NetConfig) {
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

    logln!("[Akryon Net] Initialized modern network stack on eth0 (MAC: {}, IP: {})",
        format_mac(&cfg.mac), format_ip(&cfg.ip));

    unsafe {
        *NET_STATE.0.get() = Some(cfg);
    }
}

pub fn send_raw(bytes: &[u8]) -> Result<(), &'static str> {
    if !is_card_active() {
        return Err("RTL8139 inactive");
    }
    let res = unsafe { rtl8139_send_packet(bytes.as_ptr(), bytes.len() as u32) };
    if res < 0 {
        Err("Failed to send frame")
    } else {
        Ok(())
    }
}

pub fn ipv4_send(dst_ip: Ipv4Address, proto: u8, payload: Vec<u8>) -> Result<(), &'static str> {
    let cfg = match get_config() {
        Some(c) => c,
        None => return Err("Network uninitialized"),
    };

    let next_hop = ipv4::route_destination(dst_ip, cfg.ip, cfg.netmask, cfg.gateway);
    let dest_mac = if dst_ip == [255, 255, 255, 255] || next_hop == [255, 255, 255, 255] {
        BROADCAST_MAC
    } else {
        match arp::resolve(next_hop, 1000) {
            Ok(m) => m,
            Err(_) => BROADCAST_MAC,
        }
    };

    let ip_pkt = ipv4::Ipv4Packet {
        tos: 0,
        id: 0x1234,
        flags_frag: 0x4000,
        ttl: 64,
        proto,
        src: cfg.ip,
        dst: dst_ip,
        payload,
    };

    let frame = ethernet::EthernetFrame {
        dest: dest_mac,
        src: cfg.mac,
        ethertype: ETHERTYPE_IPV4,
        payload: ip_pkt.to_bytes(),
    };

    send_raw(&frame.to_bytes())
}

pub fn poll() {
    if !is_card_active() {
        return;
    }

    let cfg = match get_config() {
        Some(c) => c,
        None => return,
    };

    let mut rx_buf = [0u8; 1536];

    while unsafe { rtl8139_has_packet() != 0 } {
        let n = unsafe { rtl8139_receive_packet(rx_buf.as_mut_ptr(), 1536) };
        if n <= 0 {
            break;
        }

        let frame = match ethernet::EthernetFrame::parse(&rx_buf[..n as usize]) {
            Some(f) => f,
            None => continue,
        };

        match frame.ethertype {
            ETHERTYPE_ARP => {
                if let Some(arp_pkt) = arp::ArpPacket::parse(&frame.payload) {
                    if let Some(reply_bytes) = arp::handle_packet(&arp_pkt, &cfg.mac, &cfg.ip) {
                        let _ = send_raw(&reply_bytes);
                    }
                }
            }
            ETHERTYPE_IPV4 => {
                if let Some(ip_pkt) = ipv4::Ipv4Packet::parse(&frame.payload) {
                    let is_for_us = ip_pkt.dst == cfg.ip
                        || ip_pkt.dst == [255, 255, 255, 255]
                        || (ip_pkt.dst[3] == 255 && ip_pkt.dst[0..3] == cfg.ip[0..3]);

                    if is_for_us {
                        match ip_pkt.proto {
                            PROTO_ICMP => icmp::handle_packet(&ip_pkt, &cfg.ip),
                            PROTO_UDP => udp::handle_packet(&ip_pkt),
                            PROTO_TCP => tcp::handle_packet(&ip_pkt),
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
