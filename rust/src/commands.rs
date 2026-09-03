use crate::vga::{self, Color};
use crate::{print, println, print_colored, logln};

extern "C" {
    fn timer_get_ticks() -> u32;
    fn timer_get_uptime_seconds() -> u32;
    fn timer_get_uptime_ms() -> u32;
    fn outb(port: u16, val: u8);
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
        "panic" => cmd_panic(args),
        "reboot" => cmd_reboot(),
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
    println!("  ls                - List files in virtual filesystem (VFS)");
    println!("  cat <file>        - Display contents of a file");
    println!("  touch <file>      - Create empty file");
    println!("  write <file> <tx> - Write text to a file");
    println!("  syscall           - Test Unix int 0x80 system call");
    println!("  echo <text>       - Print text to screen");
    println!("  color <fg> <bg>   - Change console color (0..15)");
    println!("  calc <a op b>     - Integer calculator");
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

    print_colored!(Color::LightCyan, Color::Black, "Memory Info:\n");
    println!("  Total : {} KB ({} MB)", total, total / 1024);
    println!("  Used  : {} KB ({} MB)", used, used / 1024);
    println!("  Free  : {} KB ({} MB)", free, free / 1024);
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
