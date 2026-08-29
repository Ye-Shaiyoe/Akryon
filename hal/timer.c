#include "timer.h"
#include "isr.h"
#include "io.h"

static volatile uint32_t timer_ticks = 0;

static void timer_callback(registers_t* regs) {
    (void)regs;
    timer_ticks++;
}

void timer_init(uint32_t freq) {
    if (freq == 0) freq = 100;

    isr_register_handler(32, timer_callback); // IRQ 0 = Vector 32

    // The value we send to the PIT is the value to divide it's input clock
    // (1193182 Hz) by, to get our required frequency.
    uint32_t divisor = 1193182 / freq;

    // Send the command byte (0x36 = Channel 0, lobyte/hibyte, rate generator)
    outb(0x43, 0x36);

    // Divisor must be sent byte-wise, so split here into upper/lower bytes.
    uint8_t l = (uint8_t)(divisor & 0xFF);
    uint8_t h = (uint8_t)((divisor >> 8) & 0xFF);

    outb(0x40, l);
    outb(0x40, h);
}

uint32_t timer_get_ticks(void) {
    return timer_ticks;
}

uint32_t timer_get_uptime_seconds(void) {
    return timer_ticks / TIMER_FREQUENCY_HZ;
}

uint32_t timer_get_uptime_ms(void) {
    return (timer_ticks * 1000) / TIMER_FREQUENCY_HZ;
}

void timer_sleep_ms(uint32_t ms) {
    uint32_t target_ticks = timer_ticks + (ms * TIMER_FREQUENCY_HZ) / 1000;
    while (timer_ticks < target_ticks) {
        hlt();
    }
}
