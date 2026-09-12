use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use super::dns;
use super::tcp;
use super::ipv4;

pub fn parse_url(url: &str) -> (String, u16, String) {
    let mut s = url.trim();
    if s.starts_with("http://") {
        s = &s[7..];
    }

    let mut path = String::from("/");
    let host_port = if let Some(slash_idx) = s.find('/') {
        path = String::from(&s[slash_idx..]);
        &s[..slash_idx]
    } else {
        s
    };

    let mut host = host_port;
    let mut port = 80u16;

    if let Some(colon_idx) = host_port.find(':') {
        host = &host_port[..colon_idx];
        if let Ok(p) = host_port[colon_idx + 1..].parse::<u16>() {
            port = p;
        }
    }

    (String::from(host), port, path)
}

pub fn fetch(url: &str, timeout_ms: u32) -> Result<String, &'static str> {
    let (host, port, path) = parse_url(url);

    let ip = match dns::resolve(&host, 3000) {
        Ok(addr) => addr,
        Err(_) => return Err("Failed to resolve host"),
    };

    let mut stream = tcp::connect(ip, port, timeout_ms)?;

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: AkryonOS/0.1\r\nAccept: */*\r\nConnection: close\r\n\r\n",
        path, host
    );

    stream.write(request.as_bytes())?;

    let mut body = Vec::new();
    let start_read = unsafe { super::timer_get_uptime_ms() };

    while (unsafe { super::timer_get_uptime_ms() } - start_read) < timeout_ms {
        let chunk = stream.read(500)?;
        if chunk.is_empty() {
            break;
        }
        body.extend_from_slice(&chunk);
    }

    stream.close();

    String::from_utf8(body).map_err(|_| "Response contains invalid UTF-8")
}

pub fn handle_http_request(req_text: &str) -> String {
    let first_line = req_text.lines().next().unwrap_or("");
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");

    if method != "GET" {
        return String::from("HTTP/1.1 405 Method Not Allowed\r\nConnection: close\r\n\r\n");
    }

    if path == "/" || path == "/index.html" {
        let cfg = super::get_config();
        let ip_str = cfg.map(|c| ipv4::format_ip(&c.ip)).unwrap_or_else(|| String::from("0.0.0.0"));
        let mac_str = cfg.map(|c| super::ethernet::format_mac(&c.mac)).unwrap_or_else(|| String::from("00:00:00:00:00:00"));
        let uptime_ms = unsafe { super::timer_get_uptime_ms() };

        let body = format!(
            "<!DOCTYPE html><html><head><title>Akryon OS Server</title>\
            <style>body{{background:#0d1117;color:#58a6ff;font-family:sans-serif;padding:2rem;}}\
            .box{{background:#161b22;border:1px solid #30363d;border-radius:8px;padding:1.5rem;color:#c9d1d9;}}\
            h1{{color:#7ee787;margin-top:0;}}</style></head>\
            <body><div class=\"box\"><h1>Akryon Operating System</h1>\
            <p><strong>Kernel:</strong> x86 Protected Mode Hybrid (C/Rust)</p>\
            <p><strong>IPv4 Address:</strong> {} | <strong>MAC:</strong> {}</p>\
            <p><strong>Uptime:</strong> {} ms</p>\
            <p><em>Served directly from Akryon OS HTTP Stack!</em></p>\
            </div></body></html>",
            ip_str, mac_str, uptime_ms
        );

        return format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(), body
        );
    }

    if path == "/api/status" {
        let uptime = unsafe { super::timer_get_uptime_ms() };
        let body = format!(
            "{{\"os\":\"Akryon\",\"version\":\"0.1.0\",\"arch\":\"x86\",\"uptime_ms\":{}}}",
            uptime
        );
        return format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(), body
        );
    }

    // Attempt to serve from VFS
    if let Some(file_data) = crate::vfs::read_file(path) {
        let content_type = if path.ends_with(".html") { "text/html" } else { "text/plain" };
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            content_type, file_data.len()
        );
        if let Ok(text) = core::str::from_utf8(&file_data) {
            return format!("{}{}", header, text);
        }
    }

    let not_found = "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 13\r\nConnection: close\r\n\r\n404 Not Found";
    String::from(not_found)
}
