#include "rtc.h"
#include "io.h"

// CMOS I/O ports
#define CMOS_ADDRESS_PORT  0x70
#define CMOS_DATA_PORT     0x71

// CMOS register indices
#define CMOS_REG_SECOND    0x00
#define CMOS_REG_MINUTE    0x02
#define CMOS_REG_HOUR      0x04
#define CMOS_REG_DAY       0x07
#define CMOS_REG_MONTH     0x08
#define CMOS_REG_YEAR      0x09
#define CMOS_REG_CENTURY   0x32
#define CMOS_REG_STATUS_A  0x0A
#define CMOS_REG_STATUS_B  0x0B

// Status B flags
#define STATUS_B_24H_MODE  0x02  // Bit 1: 24-hour mode (if set, 24h; else 12h)
#define STATUS_B_BINARY    0x04  // Bit 2: Binary mode (if set, binary; else BCD)
#define STATUS_A_UIP       0x80  // Bit 7: Update-In-Progress flag

// Global flags detected during init
static bool rtc_is_binary = false;
static bool rtc_is_24h    = true;

/**
 * Read a single CMOS register via port I/O.
 * NMI disabled during access (bit 7 of address port = 1).
 */
static uint8_t cmos_read(uint8_t reg) {
    outb(CMOS_ADDRESS_PORT, (uint8_t)(0x80 | reg));
    return inb(CMOS_DATA_PORT);
}

/**
 * Convert BCD byte to binary integer.
 * e.g. 0x26 -> 26, 0x59 -> 59
 */
static uint8_t bcd_to_bin(uint8_t bcd) {
    return (uint8_t)(((bcd >> 4) * 10) + (bcd & 0x0F));
}

/**
 * Wait until the Update-In-Progress bit in Status Register A clears.
 * This prevents reading inconsistent time values mid-update.
 */
static void rtc_wait_ready(void) {
    // Wait for UIP to be set (update starting) then for it to clear
    // In practice, just wait until UIP is clear
    uint32_t timeout = 100000;
    while ((cmos_read(CMOS_REG_STATUS_A) & STATUS_A_UIP) && timeout > 0) {
        timeout--;
    }
}

void rtc_init(void) {
    uint8_t status_b = cmos_read(CMOS_REG_STATUS_B);
    rtc_is_binary = (status_b & STATUS_B_BINARY) != 0;
    rtc_is_24h    = (status_b & STATUS_B_24H_MODE) != 0;
}

void rtc_get_datetime(rtc_time_t* t) {
    // Wait for CMOS to be stable (not in middle of update)
    rtc_wait_ready();

    uint8_t second  = cmos_read(CMOS_REG_SECOND);
    uint8_t minute  = cmos_read(CMOS_REG_MINUTE);
    uint8_t hour    = cmos_read(CMOS_REG_HOUR);
    uint8_t day     = cmos_read(CMOS_REG_DAY);
    uint8_t month   = cmos_read(CMOS_REG_MONTH);
    uint8_t year    = cmos_read(CMOS_REG_YEAR);
    uint8_t century = cmos_read(CMOS_REG_CENTURY);

    // Convert BCD to binary if needed
    if (!rtc_is_binary) {
        second  = bcd_to_bin(second);
        minute  = bcd_to_bin(minute);
        // Hour bit 7 is PM flag in 12h BCD mode; mask it off before converting
        uint8_t pm_flag = (!rtc_is_24h) ? (hour & 0x80) : 0;
        hour    = bcd_to_bin((uint8_t)(hour & 0x7F));
        day     = bcd_to_bin(day);
        month   = bcd_to_bin(month);
        year    = bcd_to_bin(year);
        century = bcd_to_bin(century);

        // Convert 12h to 24h if necessary
        if (!rtc_is_24h) {
            if (pm_flag && hour != 12) {
                hour = (uint8_t)(hour + 12);
            } else if (!pm_flag && hour == 12) {
                hour = 0;
            }
        }
    } else {
        // Binary mode: handle 12h PM flag
        if (!rtc_is_24h) {
            uint8_t pm_flag = hour & 0x80;
            hour = (uint8_t)(hour & 0x7F);
            if (pm_flag && hour != 12) {
                hour = (uint8_t)(hour + 12);
            } else if (!pm_flag && hour == 12) {
                hour = 0;
            }
        }
    }

    // Compute full 4-digit year
    uint16_t full_year;
    if (century != 0) {
        full_year = (uint16_t)((uint16_t)century * 100 + year);
    } else {
        // Fallback: assume 2000s
        full_year = (uint16_t)(2000 + year);
    }

    t->second = second;
    t->minute = minute;
    t->hour   = hour;
    t->day    = day;
    t->month  = month;
    t->year   = full_year;
}
