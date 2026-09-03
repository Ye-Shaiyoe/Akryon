#![no_std]
#![no_main]

extern crate alloc;

pub mod vga;
pub mod serial;
pub mod commands;
pub mod shell;
pub mod pmm;
pub mod heap;
pub mod syscall;
pub mod vfs;

use core::panic::PanicInfo;
use vga::Color;

#[global_allocator]
static ALLOCATOR: heap::KernelAllocator = heap::KernelAllocator::empty();

extern "C" {
    static kernel_start: u8;
    static kernel_end: u8;
}

#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

#[no_mangle]
pub extern "C" fn _Unwind_Resume() -> ! {
    loop {
        unsafe {
            core::arch::asm!("cli; hlt");
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga::set_color(Color::White, Color::Red);
    println!("\n[KERNEL PANIC]");
    if let Some(location) = info.location() {
        println!("Location: {}:{}", location.file(), location.line());
    }
    println!("Message: {}", info.message());
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
    vga::clear_screen();

    let k_start = unsafe { &kernel_start as *const u8 as usize };
    let k_end = unsafe { &kernel_end as *const u8 as usize };

    pmm::init(32 * 1024 * 1024, k_start, k_end);

    let heap_size = 4 * 1024 * 1024;
    let heap_start = (k_end + 4095) & !4095;
    let heap_end = heap_start + heap_size;

    let heap_start_page = heap_start / pmm::PAGE_SIZE;
    let heap_end_page = heap_end / pmm::PAGE_SIZE;
    for page in heap_start_page..heap_end_page {
        pmm::reserve_frame(page * pmm::PAGE_SIZE);
    }

    unsafe {
        ALLOCATOR.init(heap_start, heap_size);
    }

    syscall::init();
    vfs::init();

    print_colored!(Color::LightCyan, Color::Black, "    ___    __                            ____  _____\n");
    print_colored!(Color::LightCyan, Color::Black, "   /   |  / /___________  ______  ____  / __ \\/ ___/\n");
    print_colored!(Color::LightCyan, Color::Black, "  / /| | / //_/ ___/ / / / __ \\/ __ \\/ / / /\\__ \\ \n");
    print_colored!(Color::LightGreen, Color::Black, " / ___ |/ ,< / /  / /_/ / /_/ / / / / /_/ /___/ / \n");
    print_colored!(Color::LightGreen, Color::Black, "/_/  |_/_/|_/_/   \\__, /\\____/_/ /_/\\____//____/  \n");
    print_colored!(Color::LightGreen, Color::Black, "                 /____/                           \n\n");

    print_colored!(Color::Yellow, Color::Black, " Akryon Operating System - Unix-like Hybrid Architecture\n");
    println!(" -------------------------------------------------------------");
    print_colored!(Color::LightGray, Color::Black, " * Low-Level HAL & Drivers : C / Assembly (NASM)\n");
    print_colored!(Color::LightGray, Color::Black, " * Kernel Core & Shell     : Rust (no_std, alloc)\n");
    print_colored!(Color::LightGray, Color::Black, " * Target Architecture     : x86 (32-bit Protected Mode)\n");
    println!(" -------------------------------------------------------------\n");

    print_colored!(Color::LightGreen, Color::Black, "[OK] ");
    println!("Physical memory and kernel heap initialized.");
    print_colored!(Color::LightGreen, Color::Black, "[OK] ");
    println!("Unix System Calls (int 0x80) and VFS initialized.");
    print_colored!(Color::LightGreen, Color::Black, "[OK] ");
    println!("Hardware components initialized successfully.");
    print_colored!(Color::LightCyan, Color::Black, "[INFO] ");
    println!("Type 'help' for available commands or 'about' for details.\n");

    logln!("[Akryon Kernel] Rust core initialized.");
    crate::serial::log_mem(pmm::total_memory() / 1024, pmm::free_memory() / 1024);

    let test_msg = "POSIX syscall test verified at boot.\n";
    unsafe {
        core::arch::asm!(
            "int 0x80",
            in("eax") 4u32, // SYS_WRITE
            in("ebx") 1u32, // fd 1
            in("ecx") test_msg.as_ptr() as u32,
            in("edx") test_msg.len() as u32,
        );
    }

    shell::run_shell();
}

