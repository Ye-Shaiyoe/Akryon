#include "gdt.h"

extern void load_gdt_asm(uint32_t gdt_ptr);

static gdt_entry_t gdt_entries[5];
static gdt_ptr_t   gdt_ptr;

void gdt_set_gate(int32_t num, uint32_t base, uint32_t limit, uint8_t access, uint8_t gran) {
    gdt_entries[num].base_low    = (base & 0xFFFF);
    gdt_entries[num].base_middle = (base >> 16) & 0xFF;
    gdt_entries[num].base_high   = (base >> 24) & 0xFF;

    gdt_entries[num].limit_low   = (limit & 0xFFFF);
    gdt_entries[num].granularity = (limit >> 16) & 0x0F;

    gdt_entries[num].granularity |= gran & 0xF0;
    gdt_entries[num].access      = access;
}

void gdt_init(void) {
    gdt_ptr.limit = (sizeof(gdt_entry_t) * 5) - 1;
    gdt_ptr.base  = (uint32_t)&gdt_entries;

    // 0: Null descriptor
    gdt_set_gate(0, 0, 0, 0, 0);

    // 1: Kernel Code Segment (0x08) - Base=0, Limit=4GB, 32-bit, Ring 0, Exec/Read
    gdt_set_gate(1, 0, 0xFFFFFFFF, 0x9A, 0xCF);

    // 2: Kernel Data Segment (0x10) - Base=0, Limit=4GB, 32-bit, Ring 0, Read/Write
    gdt_set_gate(2, 0, 0xFFFFFFFF, 0x92, 0xCF);

    // 3: User Code Segment (0x18) - Base=0, Limit=4GB, 32-bit, Ring 3, Exec/Read
    gdt_set_gate(3, 0, 0xFFFFFFFF, 0xFA, 0xCF);

    // 4: User Data Segment (0x20) - Base=0, Limit=4GB, 32-bit, Ring 3, Read/Write
    gdt_set_gate(4, 0, 0xFFFFFFFF, 0xF2, 0xCF);

    load_gdt_asm((uint32_t)&gdt_ptr);
}
