#include "../hal/hal.h"

// Rust entry point declaration
extern void akryon_rust_main(void);

void hal_init(void) {
    // 1. Initialize Global Descriptor Table
    gdt_init();

    // 2. Initialize Interrupt Descriptor Table & Remap PIC 8259
    idt_init();

    // 3. Initialize VGA text mode console
    vga_init();

    // 4. Initialize Serial Port COM1 (38400 baud) for debug logging
    serial_init();
    serial_puts("\n[Akryon Kernel] Serial COM1 logging initialized.\n");

    // 5. Initialize PIT Timer (100Hz)
    timer_init(100);
    serial_puts("[Akryon Kernel] PIT Timer initialized (100Hz).\n");

    // 6. Initialize PS/2 Keyboard Driver
    keyboard_init();
    serial_puts("[Akryon Kernel] PS/2 Keyboard driver initialized.\n");

    // 7. Enable Interrupts (STI)
    sti();
    serial_puts("[Akryon Kernel] Hardware interrupts enabled (STI).\n");
}

void kmain(void) {
    // Inisialisasi Hardware Abstraction Layer
    hal_init();

    serial_puts("[Akryon Kernel] HAL initialization complete. Handing over to Rust Kernel Core...\n");

    // Masuk ke Rust Kernel Core & Shell
    akryon_rust_main();

    // Jika shell Rust selesai/keluar, masuk ke mode idle CPU halt
    serial_puts("[Akryon Kernel] Rust main returned. System entering idle state.\n");
    while (1) {
        hlt();
    }
}
