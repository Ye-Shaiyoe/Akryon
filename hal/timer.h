#ifndef AKRYON_HAL_TIMER_H
#define AKRYON_HAL_TIMER_H

#include "types.h"

#define TIMER_FREQUENCY_HZ 100 // 100 Hz = 10ms per tick

void timer_init(uint32_t freq);
uint32_t timer_get_ticks(void);
uint32_t timer_get_uptime_seconds(void);
uint32_t timer_get_uptime_ms(void);
void timer_sleep_ms(uint32_t ms);

#endif // AKRYON_HAL_TIMER_H
