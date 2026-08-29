#ifndef AKRYON_HAL_H
#define AKRYON_HAL_H

#include "io.h"
#include "vga.h"
#include "gdt.h"
#include "idt.h"
#include "isr.h"
#include "timer.h"
#include "keyboard.h"
#include "serial.h"

void hal_init(void);

#endif // AKRYON_HAL_H
