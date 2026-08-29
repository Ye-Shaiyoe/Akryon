#include "isr.h"
#include "vga.h"
#include "io.h"

static isr_t interrupt_handlers[256] = {0};

static const char* const exception_messages[32] = {
    "Divide by Zero",
    "Debug",
    "Non-Maskable Interrupt",
    "Breakpoint",
    "Overflow",
    "Bound Range Exceeded",
    "Invalid Opcode",
    "Device Not Available",
    "Double Fault",
    "Coprocessor Segment Overrun",
    "Invalid TSS",
    "Segment Not Present",
    "Stack-Segment Fault",
    "General Protection Fault",
    "Page Fault",
    "Reserved",
    "x87 Floating-Point Exception",
    "Alignment Check",
    "Machine Check",
    "SIMD Floating-Point Exception",
    "Virtualization Exception",
    "Control Protection Exception",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Reserved",
    "Hypervisor Injection Exception",
    "VMM Communication Exception",
    "Security Exception",
    "Reserved"
};

void isr_register_handler(uint8_t n, isr_t handler) {
    interrupt_handlers[n] = handler;
}

void isr_handler(registers_t* regs) {
    if (interrupt_handlers[regs->int_no] != 0) {
        isr_t handler = interrupt_handlers[regs->int_no];
        handler(regs);
        return;
    }

    // Unhandled CPU Exception -> Kernel Panic
    vga_set_color(VGA_COLOR_WHITE, VGA_COLOR_RED);
    vga_puts("\n\n==================== [ AKRYON KERNEL PANIC ] ====================\n");
    vga_puts(" CPU Exception: ");
    if (regs->int_no < 32) {
        vga_puts(exception_messages[regs->int_no]);
    } else {
        vga_puts("Unknown Interrupt");
    }
    vga_puts(" (Vector: ");
    vga_putdec(regs->int_no);
    vga_puts(", ErrCode: ");
    vga_puthex(regs->err_code);
    vga_puts(")\n");

    vga_puts(" EIP: "); vga_puthex(regs->eip);
    vga_puts(" CS: ");  vga_puthex(regs->cs);
    vga_puts(" EFLAGS: "); vga_puthex(regs->eflags);
    vga_puts("\n");

    vga_puts(" EAX: "); vga_puthex(regs->eax);
    vga_puts(" EBX: "); vga_puthex(regs->ebx);
    vga_puts(" ECX: "); vga_puthex(regs->ecx);
    vga_puts(" EDX: "); vga_puthex(regs->edx);
    vga_puts("\n");

    vga_puts(" ESP: "); vga_puthex(regs->esp);
    vga_puts(" EBP: "); vga_puthex(regs->ebp);
    vga_puts(" ESI: "); vga_puthex(regs->esi);
    vga_puts(" EDI: "); vga_puthex(regs->edi);
    vga_puts("\n=================================================================\n");
    vga_puts("System halted. Please reset the computer.");

    cli();
    while (1) {
        hlt();
    }
}

void irq_handler(registers_t* regs) {
    // Send EOI (End of Interrupt) to PICs
    if (regs->int_no >= 40) {
        // Send reset signal to slave
        outb(0xA0, 0x20);
    }
    // Send reset signal to master
    outb(0x20, 0x20);

    if (interrupt_handlers[regs->int_no] != 0) {
        isr_t handler = interrupt_handlers[regs->int_no];
        handler(regs);
    }
}
