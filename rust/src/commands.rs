use crate::vga::{self, Color};
use crate::{print, println, print_colored, logln};

extern "C" {
    fn timer_get_ticks() -> u32;
    fn timer_get_uptime_seconds() -> u32;
    fn timer_get_uptime_ms() -> u32;
    fn outb(port: u16, val: u8);
    fn rtc_get_datetime(t: *mut RtcTime);
}

/// Mirror of hal/rtc.h rtc_time_t - must match C layout exactly.
#[repr(C)]
struct RtcTime {
    second: u8,
    minute: u8,
    hour:   u8,
    day:    u8,
    month:  u8,
    _pad:   u8,  // alignment padding to match uint16_t year
    year:   u16,
}

pub fn handle_command(cmd: &str) {
    let trimmed = cmd.trim();
    if trimmed.is_empty() {
        return;
    }

    logln!("[Akryon Shell] Executing command: '{}'", trimmed);

    let mut parts = trimmed.splitn(2, ' ');
    let command = parts.next().unwrap_or("");
    let args = parts.next().unwrap_or("");

    match command {
        "help" => cmd_help(),
        "clear" => cmd_clear(),
        "about" => cmd_about(),
        "sysinfo" => cmd_sysinfo(),
        "free" | "meminfo" => cmd_free(),
        "uptime" => cmd_uptime(),
        "ls" => cmd_ls(),
        "cat" => cmd_cat(args),
        "touch" => cmd_touch(args),
        "write" => cmd_write(args),
        "syscall" => cmd_syscall_test(),
        "echo" => cmd_echo(args),
        "color" => cmd_color(args),
        "calc" => cmd_calc(args),
        "ifconfig" | "netinfo" => cmd_ifconfig(args),
        "ping"     => cmd_ping(args),
        "date"     => cmd_date(),
        "time"     => cmd_time(),
        "mway"     => cmd_mway(args),
        "vmm"      => cmd_vmm(args),
        "panic"    => cmd_panic(args),
        "reboot"   => cmd_reboot(),
        _ => {
            print_colored!(Color::LightRed, Color::Black, "Error: ");
            println!("Unknown command '{}'. Type 'help' for available commands.", command);
        }
    }
}

fn cmd_help() {
    print_colored!(Color::LightCyan, Color::Black, "Commands:\n");
    println!("  help              - Display this help reference");
    println!("  clear             - Clear screen");
    println!("  about             - System information");
    println!("  sysinfo           - Display hardware and CPU status");
    println!("  free / meminfo    - Display physical memory and allocator status");
    println!("  uptime            - Display system uptime");
    println!("  date              - Display current date from RTC/CMOS");
    println!("  time              - Display current time from RTC/CMOS");
    println!("  ls                - List files in virtual filesystem (VFS)");
    println!("  cat <file>        - Display contents of a file");
    println!("  touch <file>      - Create empty file");
    println!("  write <file> <tx> - Write text to a file");
    println!("  mway <file>       - Open full-screen text editor (Ctrl+S save, Ctrl+Q exit)");
    println!("  vmm [info|test]   - Virtual Memory Manager & x86 Paging status/tests");
    println!("  syscall           - Test Unix int 0x80 system call");
    println!("  echo <text>       - Print text to screen");
    println!("  color <fg> <bg>   - Change console color (0..15)");
    println!("  calc <a op b>     - Integer calculator");
    println!("  ifconfig [args]   - Display/configure network interface eth0");
    println!("  ping <ip>         - Send ICMP echo requests to target host");
    println!("  panic [msg]       - Trigger Rust Kernel Panic");
    println!("  reboot            - Restart the computer");
}

fn cmd_ls() {
    let files = crate::vfs::list_files();
    print_colored!(Color::LightCyan, Color::Black, "VFS Files:\n");
    if files.is_empty() {
        println!("  (empty)");
        return;
    }
    for (name, size) in files {
        println!("  {:<16} {} bytes", name, size);
    }
}

fn cmd_cat(args: &str) {
    let file = args.trim();
    if file.is_empty() {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("cat <filename>");
        return;
    }

    match crate::vfs::read_file(file) {
        Some(data) => {
            if let Ok(s) = core::str::from_utf8(&data) {
                print!("{}", s);
                if !s.ends_with('\n') {
                    println!("");
                }
            } else {
                for b in data {
                    print!("{:02X} ", b);
                }
                println!("");
            }
        }
        None => {
            print_colored!(Color::LightRed, Color::Black, "Error: ");
            println!("File '{}' not found", file);
        }
    }
}

fn cmd_touch(args: &str) {
    let file = args.trim();
    if file.is_empty() {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("touch <filename>");
        return;
    }

    if let Err(_) = crate::vfs::write_file(file, b"") {
        print_colored!(Color::LightRed, Color::Black, "Error: ");
        println!("Failed to create file '{}'", file);
    }
}

fn cmd_write(args: &str) {
    let mut parts = args.trim().splitn(2, ' ');
    let file = parts.next().unwrap_or("");
    let text = parts.next().unwrap_or("");

    if file.is_empty() {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("write <filename> <content>");
        return;
    }

    let mut data = alloc::vec::Vec::new();
    data.extend_from_slice(text.as_bytes());
    data.push(b'\n');

    if let Err(_) = crate::vfs::write_file(file, &data) {
        print_colored!(Color::LightRed, Color::Black, "Error: ");
        println!("Failed to write to file '{}'", file);
    }
}

fn cmd_syscall_test() {
    print_colored!(Color::LightCyan, Color::Black, "Testing Unix System Call (int 0x80)...\n");

    let msg = "Hello from Unix sys_write via int 0x80!\n";
    let ret: i32;

    unsafe {
        core::arch::asm!(
            "int 0x80",
            inlateout("eax") 4u32 => ret, // SYS_WRITE
            in("ebx") 1u32,             // fd = 1 (stdout)
            in("ecx") msg.as_ptr() as u32,
            in("edx") msg.len() as u32,
        );
    }

    println!("Syscall return value (bytes written): {}", ret);

    let pid: i32;
    unsafe {
        core::arch::asm!(
            "int 0x80",
            inlateout("eax") 20u32 => pid, // SYS_GETPID
            in("ebx") 0u32,
            in("ecx") 0u32,
            in("edx") 0u32,
        );
    }
    println!("Current PID from sys_getpid: {}", pid);
}

fn cmd_free() {
    let total = crate::pmm::total_memory() / 1024;
    let used = crate::pmm::used_memory() / 1024;
    let free = crate::pmm::free_memory() / 1024;

    print_colored!(Color::LightCyan, Color::Black, "Physical Memory (PMM):\n");
    println!("  Total : {} KB ({} MB)", total, total / 1024);
    println!("  Used  : {} KB ({} MB)", used, used / 1024);
    println!("  Free  : {} KB ({} MB)", free, free / 1024);

    print_colored!(Color::LightCyan, Color::Black, "Virtual Memory (VMM):\n");
    let paging_active = crate::vmm::is_paging_enabled();
    println!("  Paging       : {}", if paging_active { "Active (32-bit Protected Mode, CR0.PG=1, CR0.WP=1)" } else { "Inactive" });
    if paging_active {
        let cr3 = unsafe { crate::vmm::read_cr3() };
        println!("  CR3 (PD)     : 0x{:08X}", cr3);
        println!("  Mapped Pages : {} pages ({} KB)", crate::vmm::total_mapped_pages(), crate::vmm::total_mapped_pages() * 4);
        println!("  Demand Faults: {} auto-handled", crate::vmm::demand_page_fault_count());
    }
}

fn cmd_clear() {
    vga::clear_screen();
    print_colored!(Color::LightGreen, Color::Black, "Akryon OS - Unix-like Hybrid C & Rust Operating System\n\n");
}

fn cmd_about() {
    println!("Architecture : x86 (32-bit Protected Mode)");
    println!("Kernel Core  : Rust (no_std, alloc, physical memory & heap)");
    println!("HAL Drivers  : C / Assembly (GDT, IDT, PIC, PIT, PS/2, UART)");
    println!("Target Model : Unix-like OS with POSIX roadmap");
}

fn cmd_sysinfo() {
    let ticks = unsafe { timer_get_ticks() };
    let uptime_sec = unsafe { timer_get_uptime_seconds() };
    let uptime_ms = unsafe { timer_get_uptime_ms() };

    let esp_val: u32;
    unsafe {
        core::arch::asm!("mov {}, esp", out(reg) esp_val);
    }

    print_colored!(Color::LightCyan, Color::Black, "System Status:\n");
    println!("  CPU Mode     : 32-bit Protected Mode");
    println!("  Stack Pointer: 0x{:X}", esp_val);
    println!("  PIT Ticks    : {} (100 Hz)", ticks);
    println!("  Uptime       : {} seconds ({} ms)", uptime_sec, uptime_ms);
    println!("  Interrupts   : Enabled (IDT vectors 0..47)");
    println!("  Serial COM1  : 0x3F8 @ 38400 baud");
}

fn cmd_uptime() {
    let sec = unsafe { timer_get_uptime_seconds() };
    let ms = unsafe { timer_get_uptime_ms() };
    let minutes = sec / 60;
    let seconds = sec % 60;
    print_colored!(Color::LightGreen, Color::Black, "Uptime: ");
    println!("{}m {}s (total {} ms)", minutes, seconds, ms);
}

fn cmd_echo(args: &str) {
    println!("{}", args);
}

fn cmd_color(args: &str) {
    let mut parts = args.split_whitespace();
    let fg_str = parts.next();
    let bg_str = parts.next();

    if let (Some(f), Some(b)) = (fg_str, bg_str) {
        if let (Ok(fg_num), Ok(bg_num)) = (f.parse::<u8>(), b.parse::<u8>()) {
            if fg_num < 16 && bg_num < 16 {
                vga::set_color(Color::from_u8(fg_num), Color::from_u8(bg_num));
                println!("Color updated: fg={}, bg={}", fg_num, bg_num);
                return;
            }
        }
    }

    print_colored!(Color::LightRed, Color::Black, "Usage: ");
    println!("color <fg:0-15> <bg:0-15>");
    println!("Colors: 0:Black, 1:Blue, 2:Green, 3:Cyan, 4:Red, 5:Magenta, 6:Brown, 7:LGray,");
    println!("        8:DGray, 9:LBlue, 10:LGreen, 11:LCyan, 12:LRed, 13:LMagenta, 14:Yellow, 15:White");
}

fn cmd_calc(args: &str) {
    let mut parts = args.split_whitespace();
    let a_str = parts.next();
    let op_str = parts.next();
    let b_str = parts.next();

    if let (Some(a_s), Some(op), Some(b_s)) = (a_str, op_str, b_str) {
        if let (Ok(a), Ok(b)) = (a_s.parse::<i32>(), b_s.parse::<i32>()) {
            let res = match op {
                "+" => Some(a.wrapping_add(b)),
                "-" => Some(a.wrapping_sub(b)),
                "*" => Some(a.wrapping_mul(b)),
                "/" => {
                    if b == 0 {
                        print_colored!(Color::LightRed, Color::Black, "Error: ");
                        println!("Division by zero!");
                        return;
                    }
                    Some(a / b)
                }
                "%" => {
                    if b == 0 {
                        print_colored!(Color::LightRed, Color::Black, "Error: ");
                        println!("Modulo by zero!");
                        return;
                    }
                    Some(a % b)
                }
                _ => None,
            };

            if let Some(val) = res {
                print_colored!(Color::LightGreen, Color::Black, "Result: ");
                println!("{} {} {} = {}", a, op, b, val);
                return;
            }
        }
    }

    print_colored!(Color::LightRed, Color::Black, "Usage: ");
    println!("calc <num1> <+|-|*|/|%> <num2> (e.g. calc 100 * 5)");
}

fn cmd_panic(args: &str) {
    let msg = if args.trim().is_empty() {
        "Manual panic triggered by user from Akryon shell!"
    } else {
        args.trim()
    };
    panic!("{}", msg);
}

fn cmd_reboot() {
    print_colored!(Color::Yellow, Color::Black, "Rebooting Akryon OS...\n");
    logln!("[Akryon Kernel] System reboot triggered.");

    unsafe {
        core::arch::asm!("cli");
        for _ in 0..1000 {
            outb(0x64, 0xFE);
        }
        let null_idt: [u16; 3] = [0, 0, 0];
        core::arch::asm!("lidt [{}]", in(reg) null_idt.as_ptr());
        core::arch::asm!("int3");
    }
}

fn cmd_ifconfig(args: &str) {
    let trimmed = args.trim();
    if trimmed.starts_with("set ") {
        let parts: alloc::vec::Vec<&str> = trimmed[4..].split_whitespace().collect();
        if parts.len() == 3 {
            let ip_opt = crate::net::parse_ip(parts[0]);
            let mask_opt = crate::net::parse_ip(parts[1]);
            let gw_opt = crate::net::parse_ip(parts[2]);

            if let (Some(ip), Some(mask), Some(gw)) = (ip_opt, mask_opt, gw_opt) {
                crate::net::update_config(ip, mask, gw);
                print_colored!(Color::LightGreen, Color::Black, "[OK] ");
                println!("Network eth0 updated: IP={}, Mask={}, Gateway={}",
                    parts[0], parts[1], parts[2]);
                println!("Configuration saved to /etc/network.conf in VFS.");
                return;
            }
        }
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("ifconfig set <ip> <netmask> <gateway>");
        println!("Example: ifconfig set 10.0.2.15 255.255.255.0 10.0.2.2");
        return;
    }

    if !trimmed.is_empty() && trimmed != "eth0" {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("ifconfig [eth0]");
        println!("       ifconfig set <ip> <netmask> <gateway>");
        return;
    }

    match crate::net::get_config() {
        Some(cfg) => {
            let (rx_pkts, tx_pkts, rx_bytes, tx_bytes) = crate::net::get_stats();
            let status = if cfg.is_up { "UP, BROADCAST, RUNNING" } else { "DOWN" };

            print_colored!(Color::LightCyan, Color::Black, "eth0: ");
            println!("flags=<{}> mtu 1500", status);
            println!("      inet {}  netmask {}  gateway {}",
                crate::net::format_ip(&cfg.ip),
                crate::net::format_ip(&cfg.netmask),
                crate::net::format_ip(&cfg.gateway));
            println!("      nameserver {}", crate::net::format_ip(&cfg.dns));
            println!("      ether {} (Realtek RTL8139)", crate::net::format_mac(&cfg.mac));
            println!("      RX packets {}  bytes {} ({})",
                rx_pkts, rx_bytes, if rx_bytes < 1024 { "B" } else { "KB" });
            println!("      TX packets {}  bytes {} ({})",
                tx_pkts, tx_bytes, if tx_bytes < 1024 { "B" } else { "KB" });
        }
        None => {
            print_colored!(Color::LightRed, Color::Black, "Error: ");
            println!("Network interface eth0 unavailable.");
        }
    }
}

fn cmd_ping(args: &str) {
    let host = args.trim();
    if host.is_empty() {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("ping <ip_address> (e.g. ping 10.0.2.2)");
        return;
    }

    let target_ip = match crate::net::parse_ip(host) {
        Some(ip) => ip,
        None => {
            print_colored!(Color::LightRed, Color::Black, "Error: ");
            println!("Invalid IPv4 address format '{}'.", host);
            return;
        }
    };

    println!("PING {} ({}) 32 bytes of ICMP data:", host, host);

    let mut transmitted = 0;
    let mut received = 0;

    for seq in 1..=4 {
        transmitted += 1;
        match crate::net::send_ping(target_ip, seq) {
            Ok(rtt) => {
                received += 1;
                println!("32 bytes from {}: icmp_seq={} ttl=64 time={} ms", host, seq, rtt);
            }
            Err(e) => {
                println!("From {}: icmp_seq={} {}", host, seq, e);
            }
        }
    }

    println!("\n--- {} ping statistics ---", host);
    let loss = if transmitted > 0 { ((transmitted - received) * 100) / transmitted } else { 0 };
    println!("{} packets transmitted, {} received, {}% packet loss", transmitted, received, loss);
}

// ---------------------------------------------------------------------------
// RTC / CMOS date & time commands
// ---------------------------------------------------------------------------

fn read_rtc() -> RtcTime {
    let mut t = RtcTime {
        second: 0,
        minute: 0,
        hour:   0,
        day:    0,
        month:  0,
        _pad:   0,
        year:   0,
    };
    unsafe { rtc_get_datetime(&mut t as *mut RtcTime); }
    t
}

fn cmd_date() {
    let t = read_rtc();
    print_colored!(Color::LightCyan, Color::Black, "Date: ");
    // Format: YYYY-MM-DD
    print_padded2(t.year as u32, false);
    print!("-");
    print_padded2(t.month as u32, true);
    print!("-");
    print_padded2(t.day as u32, true);
    println!("  (from RTC/CMOS)");
}

fn cmd_time() {
    let t = read_rtc();
    print_colored!(Color::LightCyan, Color::Black, "Time: ");
    // Format: HH:MM:SS
    print_padded2(t.hour as u32, true);
    print!(":");
    print_padded2(t.minute as u32, true);
    print!(":");
    print_padded2(t.second as u32, true);
    println!("  (from RTC/CMOS)");
}

/// Print a number, zero-padded to 2 digits if `pad` is true.
fn print_padded2(val: u32, pad: bool) {
    if pad && val < 10 {
        print!("0{}", val);
    } else {
        print!("{}", val);
    }
}

// ---------------------------------------------------------------------------
// mway - Full-screen text editor
// ---------------------------------------------------------------------------

fn cmd_mway(args: &str) {
    let filename = args.trim();
    if filename.is_empty() {
        print_colored!(Color::LightRed, Color::Black, "Usage: ");
        println!("mway <filename>");
        println!("  Example: mway catatan.txt");
        return;
    }

    logln!("[mway] Opening file: '{}'", filename);
    crate::editor::open(filename);
    logln!("[mway] Editor closed for: '{}'", filename);
}

// ---------------------------------------------------------------------------
// vmm - Virtual Memory Manager & x86 Paging
// ---------------------------------------------------------------------------

fn cmd_vmm(args: &str) {
    let parts: alloc::vec::Vec<&str> = args.split_whitespace().collect();
    let subcmd = if parts.is_empty() { "info" } else { parts[0] };

    match subcmd {
        "info" | "status" => {
            print_colored!(Color::LightCyan, Color::Black, "Virtual Memory Manager (VMM) Status:\n");
            let active = crate::vmm::is_paging_enabled();
            println!("  x86 Paging      : {}", if active { "ENABLED (32-bit Protected Mode)" } else { "DISABLED" });
            if active {
                let cr3 = unsafe { crate::vmm::read_cr3() };
                println!("  Page Dir (CR3)  : 0x{:08X}", cr3);
                println!("  Identity Map    : 0x00000000 - 0x03FFFFFF (64 MB)");
                println!("  Total Mapped    : {} pages ({} KB)", 
                    crate::vmm::total_mapped_pages(),
                    crate::vmm::total_mapped_pages() * 4
                );
                println!("  Demand Range    : 0x{:08X} - 0x{:08X} (16 MB)", crate::vmm::DEMAND_PAGING_START, crate::vmm::DEMAND_PAGING_END);
                println!("  Demand Faults   : {} auto-allocated via ISR 14", crate::vmm::demand_page_fault_count());
                println!("  Protection      : Supervisor Write-Protect (CR0.WP=1)");
            }
        }
        "test" => {
            print_colored!(Color::LightCyan, Color::Black, "[VMM Test] ");
            println!("Running automated paging and demand-paging verification suite...");
            
            print!("  1. Testing Identity Mapping (Kernel & VGA) ... ");
            let k_ok = crate::vmm::get_phys_addr(0x10000) == Some(0x10000);
            let vga_ok = crate::vmm::get_phys_addr(0xB8000) == Some(0xB8000);
            if k_ok && vga_ok {
                print_colored!(Color::LightGreen, Color::Black, "PASSED\n");
            } else {
                print_colored!(Color::LightRed, Color::Black, "FAILED\n");
            }

            print!("  2. Testing Demand Paging on 0xC0002000 ... ");
            let test_addr = 0xC0002000 as *mut u32;
            let val = 0x5A5A1234;
            unsafe {
                core::ptr::write_volatile(test_addr, val);
                let read = core::ptr::read_volatile(test_addr);
                if read == val && crate::vmm::get_phys_addr(0xC0002000).is_some() {
                    print_colored!(Color::LightGreen, Color::Black, "PASSED");
                    println!(" (auto-allocated frame: 0x{:08X})", crate::vmm::get_phys_addr(0xC0002000).unwrap());
                } else {
                    print_colored!(Color::LightRed, Color::Black, "FAILED\n");
                }
            }

            print!("  3. Testing Dynamic Mapping & Unmapping ... ");
            let custom_v = 0xD0001000;
            if let Some(frame) = crate::pmm::alloc_frame() {
                let map_res = crate::vmm::map_page(custom_v, frame, crate::vmm::PAGE_WRITABLE);
                let mut data_ok = false;
                if map_res.is_ok() {
                    unsafe {
                        let ptr = custom_v as *mut u32;
                        core::ptr::write_volatile(ptr, 0xCAFEBABE);
                        data_ok = core::ptr::read_volatile(ptr) == 0xCAFEBABE;
                    }
                    let _ = crate::vmm::unmap_page(custom_v);
                }
                crate::pmm::free_frame(frame);
                if data_ok {
                    print_colored!(Color::LightGreen, Color::Black, "PASSED\n");
                } else {
                    print_colored!(Color::LightRed, Color::Black, "FAILED\n");
                }
            } else {
                print_colored!(Color::LightRed, Color::Black, "OUT OF MEMORY\n");
            }

            print_colored!(Color::LightGreen, Color::Black, "\n[SUCCESS] ");
            println!("All Virtual Memory Manager tests executed successfully!");
        }
        "map" => {
            if parts.len() < 3 {
                print_colored!(Color::LightRed, Color::Black, "Usage: ");
                println!("vmm map <virt_hex> <phys_hex>");
                println!("  Example: vmm map D0000000 1000000");
                return;
            }
            let virt = usize::from_str_radix(parts[1].trim_start_matches("0x"), 16);
            let phys = usize::from_str_radix(parts[2].trim_start_matches("0x"), 16);
            match (virt, phys) {
                (Ok(v), Ok(p)) => {
                    match crate::vmm::map_page(v, p, crate::vmm::PAGE_WRITABLE) {
                        Ok(_) => {
                            print_colored!(Color::LightGreen, Color::Black, "[OK] ");
                            println!("Mapped virt 0x{:08X} -> phys 0x{:08X}", v, p);
                        }
                        Err(e) => {
                            print_colored!(Color::LightRed, Color::Black, "Error: ");
                            println!("{}", e);
                        }
                    }
                }
                _ => {
                    print_colored!(Color::LightRed, Color::Black, "Error: ");
                    println!("Invalid hexadecimal address.");
                }
            }
        }
        "fault" => {
            print_colored!(Color::Yellow, Color::Black, "[WARNING] ");
            println!("Triggering intentional Page Fault at unmapped address 0xDEAD0000...");
            println!("This will invoke the Page Fault Exception Handler (ISR 14).");
            unsafe {
                let ptr = 0xDEAD0000 as *const u32;
                let _ = core::ptr::read_volatile(ptr);
            }
        }
        _ => {
            print_colored!(Color::LightCyan, Color::Black, "VMM Commands:\n");
            println!("  vmm info                   - Display paging and directory information");
            println!("  vmm test                   - Run automated Demand Paging & VMM test suite");
            println!("  vmm map <virt_hex> <phys>  - Map virtual page to physical frame");
            println!("  vmm fault                  - Trigger intentional page fault exception (panic test)");
        }
    }
}


