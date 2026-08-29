use crate::vga::{self, Color};
use crate::{println, print_colored, logln};

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
        "uptime" => cmd_uptime(),
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
    print_colored!(Color::LightCyan, Color::Black, "==================== [ AKRYON COMMANDS ] ====================\n");
    println!("  help              - Display this help reference");
    println!("  clear             - Clear screen and display Akryon banner");
    println!("  about             - System information & hybrid C+Rust architecture");
    println!("  sysinfo           - Display hardware, ticks, and kernel status");
    println!("  uptime            - Display system uptime since boot");
    println!("  echo <text>       - Print text to screen");
    println!("  color <fg> <bg>   - Change console color (0..15)");
    println!("  calc <a op b>     - Integer calculator (e.g. 'calc 42 + 58')");
    println!("  panic [msg]       - Trigger Rust Kernel Panic test");
    println!("  reboot            - Restart the computer");
    print_colored!(Color::LightCyan, Color::Black, "=============================================================\n");
}

fn cmd_clear() {
    vga::clear_screen();
    print_colored!(Color::LightCyan, Color::Black, "=============================================================\n");
    print_colored!(Color::LightGreen, Color::Black, "        Akryon OS v2.0 - Hybrid C & Rust Operating System     \n");
    print_colored!(Color::LightCyan, Color::Black, "=============================================================\n\n");
}

fn cmd_about() {
    print_colored!(Color::LightGreen, Color::Black, "Akryon OS v2.0 (Hybrid Architecture)\n");
    println!("-------------------------------------------------------------");
    println!("* Architecture    : x86 (32-bit Protected Mode)");
    println!("* Low-Level HAL   : C & Assembly (NASM)");
    println!("  - Drivers       : GDT, IDT (PIC 8259), PIT 8254, PS/2 KBD, UART 16550");
    println!("* Core & Shell    : Rust (no_std, Safe Formatted I/O, Command Engine)");
    println!("* Bootloader      : Custom 512B MBR + Stage 2 Loader");
    println!("* Status          : Running natively on bare-metal / QEMU");
    println!("-------------------------------------------------------------");
}

fn cmd_sysinfo() {
    let ticks = unsafe { timer_get_ticks() };
    let uptime_sec = unsafe { timer_get_uptime_seconds() };
    let uptime_ms = unsafe { timer_get_uptime_ms() };

    let esp_val: u32;
    unsafe {
        core::arch::asm!("mov {}, esp", out(reg) esp_val);
    }

    print_colored!(Color::LightCyan, Color::Black, "--- [ System Status ] ---\n");
    println!("  CPU Mode     : 32-bit Protected Mode (Flat Memory)");
    println!("  Stack Pointer: 0x{:X}", esp_val);
    println!("  PIT Ticks    : {} (100 Hz frequency)", ticks);
    println!("  Uptime       : {} seconds ({} ms)", uptime_sec, uptime_ms);
    println!("  Memory Model : 4GB Flat Address Space (0x00000000 - 0xFFFFFFFF)");
    println!("  Interrupts   : Enabled (IDT vectors 0..47 active)");
    println!("  Serial Port  : COM1 (0x3F8 @ 38400 baud active)");
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
