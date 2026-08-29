#ifndef AKRYON_HAL_ISR_H
#define AKRYON_HAL_ISR_H

#include "types.h"

typedef struct registers {
    uint32_t ds;                                     // Data segment selector
    uint32_t edi, esi, ebp, esp, ebx, edx, ecx, eax; // Pushed by pusha
    uint32_t int_no, err_code;                       // Interrupt number and error code
    uint32_t eip, cs, eflags, useresp, ss;           // Pushed by the processor automatically
} registers_t;

typedef void (*isr_t)(registers_t*);

void isr_register_handler(uint8_t n, isr_t handler);
void isr_handler(registers_t* regs);
void irq_handler(registers_t* regs);

#endif // AKRYON_HAL_ISR_H
