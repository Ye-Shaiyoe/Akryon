#ifndef AKRYON_HAL_RTC_H
#define AKRYON_HAL_RTC_H

#include "types.h"

/**
 * RTC/CMOS Real-Time Clock datetime structure.
 * Fields are already converted from BCD to binary integers.
 */
typedef struct {
    uint8_t  second;  // 0..59
    uint8_t  minute;  // 0..59
    uint8_t  hour;    // 0..23
    uint8_t  day;     // 1..31
    uint8_t  month;   // 1..12
    uint16_t year;    // e.g. 2026
} rtc_time_t;

/**
 * Initialize RTC. Reads the CMOS status register B to determine
 * BCD vs binary and 12h vs 24h mode flags.
 */
void rtc_init(void);

/**
 * Read current date and time from CMOS RTC into *t.
 * Waits for Update-In-Progress (UIP) flag to clear before reading.
 */
void rtc_get_datetime(rtc_time_t* t);

#endif // AKRYON_HAL_RTC_H
