use alloc::vec::Vec;
use core::cell::UnsafeCell;
use super::ipv4::{self, Ipv4Address, Ipv4Packet, PROTO_TCP};

extern "C" {
    fn timer_get_uptime_ms() -> u32;
}

pub const TCP_FIN: u16 = 0x0001;
pub const TCP_SYN: u16 = 0x0002;
pub const TCP_RST: u16 = 0x0004;
pub const TCP_PSH: u16 = 0x0008;
pub const TCP_ACK: u16 = 0x0010;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    TimeWait,
}

#[derive(Debug, Clone)]
pub struct TcpSegment {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq: u32,
    pub ack: u32,
    pub flags: u16,
    pub window: u16,
    pub payload: Vec<u8>,
}

impl TcpSegment {
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 20 {
            return None;
        }

        let src_port = u16::from_be_bytes([data[0], data[1]]);
        let dst_port = u16::from_be_bytes([data[2], data[3]]);
        let seq = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let ack = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        let data_offset = ((data[12] >> 4) as usize) * 4;
        let flags = u16::from_be_bytes([data[12] & 0x01, data[13]]);
        let window = u16::from_be_bytes([data[14], data[15]]);

        if data.len() < data_offset {
            return None;
        }

        let payload = data[data_offset..].to_vec();

        Some(Self {
            src_port,
            dst_port,
            seq,
            ack,
            flags,
            window,
            payload,
        })
    }

    pub fn to_bytes(&self, src_ip: Ipv4Address, dst_ip: Ipv4Address) -> Vec<u8> {
        let total_len = (20 + self.payload.len()) as u16;
        let mut pseudo = Vec::with_capacity(12 + total_len as usize);

        pseudo.extend_from_slice(&src_ip);
        pseudo.extend_from_slice(&dst_ip);
        pseudo.push(0);
        pseudo.push(PROTO_TCP);
        pseudo.extend_from_slice(&total_len.to_be_bytes());

        let mut tcp_hdr = Vec::with_capacity(total_len as usize);
        tcp_hdr.extend_from_slice(&self.src_port.to_be_bytes());
        tcp_hdr.extend_from_slice(&self.dst_port.to_be_bytes());
        tcp_hdr.extend_from_slice(&self.seq.to_be_bytes());
        tcp_hdr.extend_from_slice(&self.ack.to_be_bytes());

        let doff_flags = (5u16 << 12) | (self.flags & 0x01FF);
        tcp_hdr.extend_from_slice(&doff_flags.to_be_bytes());
        tcp_hdr.extend_from_slice(&self.window.to_be_bytes());
        tcp_hdr.extend_from_slice(&[0, 0]); // Checksum placeholder
        tcp_hdr.extend_from_slice(&[0, 0]); // Urgent pointer
        tcp_hdr.extend_from_slice(&self.payload);

        pseudo.extend_from_slice(&tcp_hdr);
        let csum = ipv4::checksum(&pseudo);
        tcp_hdr[16..18].copy_from_slice(&csum.to_be_bytes());
        tcp_hdr
    }
}

pub struct TcpSocket {
    pub local_port: u16,
    pub remote_ip: Ipv4Address,
    pub remote_port: u16,
    pub state: TcpState,
    pub local_seq: u32,
    pub remote_seq: u32,
    pub rx_buf: Vec<u8>,
    pub is_listener: bool,
    pub connected: bool,
    pub peer_closed: bool,
}

const MAX_SOCKETS: usize = 16;

struct SafeTcpManager(UnsafeCell<Vec<TcpSocket>>);
unsafe impl Sync for SafeTcpManager {}

static TCP_MANAGER: SafeTcpManager = SafeTcpManager(UnsafeCell::new(Vec::new()));

fn send_segment(
    sock: &TcpSocket,
    flags: u16,
    payload: &[u8],
) -> Result<(), &'static str> {
    let cfg = match super::get_config() {
        Some(c) => c,
        None => return Err("Network uninitialized"),
    };

    let seg = TcpSegment {
        src_port: sock.local_port,
        dst_port: sock.remote_port,
        seq: sock.local_seq,
        ack: sock.remote_seq,
        flags,
        window: 8192,
        payload: payload.to_vec(),
    };

    let bytes = seg.to_bytes(cfg.ip, sock.remote_ip);
    super::ipv4_send(sock.remote_ip, PROTO_TCP, bytes)
}

pub fn handle_packet(ip_pkt: &Ipv4Packet) {
    let seg = match TcpSegment::parse(&ip_pkt.payload) {
        Some(s) => s,
        None => return,
    };

    let sockets = unsafe { &mut *TCP_MANAGER.0.get() };

    for sock in sockets.iter_mut() {
        if sock.local_port == seg.dst_port {
            if sock.state == TcpState::Listen && (seg.flags & TCP_SYN) != 0 {
                sock.remote_ip = ip_pkt.src;
                sock.remote_port = seg.src_port;
                sock.remote_seq = seg.seq.wrapping_add(1);
                sock.local_seq = 0x2048_0000;
                sock.state = TcpState::SynReceived;
                let _ = send_segment(sock, TCP_SYN | TCP_ACK, &[]);
                sock.local_seq = sock.local_seq.wrapping_add(1);
                return;
            }

            if sock.remote_ip == ip_pkt.src && sock.remote_port == seg.src_port {
                match sock.state {
                    TcpState::SynSent => {
                        if (seg.flags & (TCP_SYN | TCP_ACK)) == (TCP_SYN | TCP_ACK) {
                            sock.remote_seq = seg.seq.wrapping_add(1);
                            sock.local_seq = seg.ack;
                            sock.state = TcpState::Established;
                            sock.connected = true;
                            let _ = send_segment(sock, TCP_ACK, &[]);
                        }
                    }
                    TcpState::SynReceived => {
                        if (seg.flags & TCP_ACK) != 0 {
                            sock.state = TcpState::Established;
                            sock.connected = true;
                        }
                    }
                    TcpState::Established => {
                        if !seg.payload.is_empty() {
                            sock.rx_buf.extend_from_slice(&seg.payload);
                            sock.remote_seq = seg.seq.wrapping_add(seg.payload.len() as u32);
                            let _ = send_segment(sock, TCP_ACK, &[]);
                        }
                        if (seg.flags & TCP_FIN) != 0 {
                            sock.remote_seq = seg.seq.wrapping_add(1);
                            sock.peer_closed = true;
                            sock.state = TcpState::CloseWait;
                            let _ = send_segment(sock, TCP_ACK, &[]);
                        }
                    }
                    TcpState::FinWait1 => {
                        if (seg.flags & TCP_ACK) != 0 {
                            sock.state = TcpState::FinWait2;
                        }
                        if (seg.flags & TCP_FIN) != 0 {
                            sock.remote_seq = seg.seq.wrapping_add(1);
                            let _ = send_segment(sock, TCP_ACK, &[]);
                            sock.state = TcpState::TimeWait;
                        }
                    }
                    TcpState::FinWait2 => {
                        if (seg.flags & TCP_FIN) != 0 {
                            sock.remote_seq = seg.seq.wrapping_add(1);
                            let _ = send_segment(sock, TCP_ACK, &[]);
                            sock.state = TcpState::Closed;
                        }
                    }
                    _ => {}
                }
                return;
            }
        }
    }
}

pub struct TcpStream {
    pub socket_id: usize,
}

static mut NEXT_CLIENT_PORT: u16 = 49152;

pub fn connect(remote_ip: Ipv4Address, remote_port: u16, timeout_ms: u32) -> Result<TcpStream, &'static str> {
    let local_port = unsafe {
        let p = NEXT_CLIENT_PORT;
        NEXT_CLIENT_PORT = if p >= 65000 { 49152 } else { p + 1 };
        p
    };

    let sockets = unsafe { &mut *TCP_MANAGER.0.get() };
    if sockets.len() >= MAX_SOCKETS {
        return Err("Too many open TCP sockets");
    }

    let sock = TcpSocket {
        local_port,
        remote_ip,
        remote_port,
        state: TcpState::SynSent,
        local_seq: 0x1000_0000,
        remote_seq: 0,
        rx_buf: Vec::new(),
        is_listener: false,
        connected: false,
        peer_closed: false,
    };

    sockets.push(sock);
    let socket_id = sockets.len() - 1;

    let _ = send_segment(&sockets[socket_id], TCP_SYN, &[]);
    sockets[socket_id].local_seq = sockets[socket_id].local_seq.wrapping_add(1);

    let start = unsafe { timer_get_uptime_ms() };
    while (unsafe { timer_get_uptime_ms() } - start) < timeout_ms {
        super::poll();
        let current_sock = &unsafe { &*TCP_MANAGER.0.get() }[socket_id];
        if current_sock.connected {
            return Ok(TcpStream { socket_id });
        }
    }

    unsafe {
        (&mut *TCP_MANAGER.0.get()).remove(socket_id);
    }
    Err("TCP connection timed out (handshake failed)")
}

impl TcpStream {
    pub fn write(&mut self, data: &[u8]) -> Result<(), &'static str> {
        let sockets = unsafe { &mut *TCP_MANAGER.0.get() };
        if self.socket_id >= sockets.len() {
            return Err("Invalid socket ID");
        }

        let sock = &mut sockets[self.socket_id];
        if sock.state != TcpState::Established {
            return Err("Socket not in ESTABLISHED state");
        }

        let _ = send_segment(sock, TCP_PSH | TCP_ACK, data);
        sock.local_seq = sock.local_seq.wrapping_add(data.len() as u32);
        Ok(())
    }

    pub fn read(&mut self, timeout_ms: u32) -> Result<Vec<u8>, &'static str> {
        let start = unsafe { timer_get_uptime_ms() };
        loop {
            super::poll();
            let sockets = unsafe { &mut *TCP_MANAGER.0.get() };
            if self.socket_id >= sockets.len() {
                return Err("Socket closed");
            }
            let sock = &mut sockets[self.socket_id];

            if !sock.rx_buf.is_empty() {
                let out = sock.rx_buf.clone();
                sock.rx_buf.clear();
                return Ok(out);
            }

            if sock.peer_closed || sock.state == TcpState::Closed || sock.state == TcpState::CloseWait {
                return Ok(Vec::new());
            }

            if (unsafe { timer_get_uptime_ms() } - start) >= timeout_ms {
                break;
            }
        }
        Ok(Vec::new())
    }

    pub fn close(&mut self) {
        let sockets = unsafe { &mut *TCP_MANAGER.0.get() };
        if self.socket_id < sockets.len() {
            let sock = &mut sockets[self.socket_id];
            let _ = send_segment(sock, TCP_FIN | TCP_ACK, &[]);
            sock.local_seq = sock.local_seq.wrapping_add(1);
            sock.state = TcpState::FinWait1;
        }
    }
}

pub fn listen(port: u16) -> Result<usize, &'static str> {
    let sockets = unsafe { &mut *TCP_MANAGER.0.get() };
    for (idx, sock) in sockets.iter().enumerate() {
        if sock.local_port == port && sock.is_listener {
            return Ok(idx);
        }
    }

    if sockets.len() >= MAX_SOCKETS {
        return Err("Socket table full");
    }

    let sock = TcpSocket {
        local_port: port,
        remote_ip: [0, 0, 0, 0],
        remote_port: 0,
        state: TcpState::Listen,
        local_seq: 0,
        remote_seq: 0,
        rx_buf: Vec::new(),
        is_listener: true,
        connected: false,
        peer_closed: false,
    };

    sockets.push(sock);
    Ok(sockets.len() - 1)
}

pub fn get_socket_count() -> usize {
    unsafe { (&*TCP_MANAGER.0.get()).len() }
}

pub fn service_http_server(port: u16) -> bool {
    let sockets = unsafe { &mut *TCP_MANAGER.0.get() };
    for sock in sockets.iter_mut() {
        if sock.local_port == port && sock.state == TcpState::Established && !sock.rx_buf.is_empty() {
            if let Ok(req_str) = core::str::from_utf8(&sock.rx_buf) {
                let resp = super::http::handle_http_request(req_str);
                let _ = send_segment(sock, TCP_PSH | TCP_ACK, resp.as_bytes());
                sock.local_seq = sock.local_seq.wrapping_add(resp.len() as u32);
                let _ = send_segment(sock, TCP_FIN | TCP_ACK, &[]);
                sock.local_seq = sock.local_seq.wrapping_add(1);
                sock.state = TcpState::FinWait1;
                sock.rx_buf.clear();
                return true;
            }
        }
    }
    false
}

pub fn close_listener(port: u16) {
    let sockets = unsafe { &mut *TCP_MANAGER.0.get() };
    sockets.retain(|sock| !(sock.local_port == port && sock.is_listener));
}
