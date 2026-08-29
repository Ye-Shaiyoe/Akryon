#ifndef AKRYON_HAL_SERIAL_H
#define AKRYON_HAL_SERIAL_H

#include "types.h"

#define COM1_PORT 0x3F8

int  serial_init(void);
void serial_putchar(char c);
void serial_puts(const char* str);
void serial_puthex(uint32_t val);
void serial_putdec(uint32_t val);

#endif // AKRYON_HAL_SERIAL_H
