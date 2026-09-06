#ifndef AKRYON_HAL_RTL8139_H
#define AKRYON_HAL_RTL8139_H

#include "types.h"

int  rtl8139_init(void);
int  rtl8139_is_active(void);
int  rtl8139_get_mac(uint8_t *mac_out);
int  rtl8139_send_packet(const void *data, uint32_t len);
int  rtl8139_receive_packet(void *buf, uint32_t max_len);
void rtl8139_get_stats(uint32_t *rx_pkts, uint32_t *tx_pkts, uint32_t *rx_bytes, uint32_t *tx_bytes);

#endif // AKRYON_HAL_RTL8139_H
