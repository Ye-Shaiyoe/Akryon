#![no_std]
#![no_main]

pub mod vga;
pub mod serial;
pub mod commands;
pub mod shell;

use core::panic::PanicInfo;
use vga::Color;

#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga::set_color(Color::White, Color::Red);
    println!("\n\n==================== [ RUST KERNEL PANIC ] ====================");
    if let Some(location) = info.location() {
        println!(" Location: {}:{}", location.file(), location.line());
    }
    println!(" Message : {}", info.message());
    println!("===============================================================");
    println!("System halted. Please reboot.");

    logln!("[Akryon Kernel Panic] {}", info.message());

    loop {
        unsafe {
            core::arch::asm!("cli; hlt");
        }
    }
}

#[no_mangle]
pub extern "C" fn akryon_rust_main() -> ! {
    // Bersihkan layar
    vga::clear_screen();

    // Print Boot Banner Akryon OS
    print_colored!(Color::LightCyan, Color::Black, "    ___    __                            ____  _____\n");
    print_colored!(Color::LightCyan, Color::Black, "   /   |  / /___________  ______  ____  / __ \\/ ___/\n");
    print_colored!(Color::LightCyan, Color::Black, "  / /| | / //_/ ___/ / / / __ \\/ __ \\/ / / /\\__ \\ \n");
    print_colored!(Color::LightGreen, Color::Black, " / ___ |/ ,< / /  / /_/ / /_/ / / / / /_/ /___/ / \n");
    print_colored!(Color::LightGreen, Color::Black, "/_/  |_/_/|_/_/   \\__, /\\____/_/ /_/\\____//____/  \n");
    print_colored!(Color::LightGreen, Color::Black, "                 /____/                           \n\n");

    print_colored!(Color::Yellow, Color::Black, " Akryon Operating System v2.0 - Hybrid C & Rust Architecture\n");
    println!(" -------------------------------------------------------------");
    print_colored!(Color::LightGray, Color::Black, " * Low-Level HAL & Drivers : C / Assembly (NASM)\n");
    print_colored!(Color::LightGray, Color::Black, " * Kernel Core & Shell     : Rust (no_std, Safe Formatted I/O)\n");
    print_colored!(Color::LightGray, Color::Black, " * Target Architecture     : x86 (32-bit Protected Mode)\n");
    println!(" -------------------------------------------------------------\n");

    print_colored!(Color::LightGreen, Color::Black, "[OK] ");
    println!("System and hardware components initialized successfully.");
    print_colored!(Color::LightCyan, Color::Black, "[INFO] ");
    println!("Type 'help' for available commands or 'about' for details.\n");

    logln!("[Akryon Kernel] Rust core initialized. Starting shell interface.");

    // Mulai interactive shell
    shell::run_shell();
}
