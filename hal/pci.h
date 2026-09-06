#ifndef AKRYON_HAL_PCI_H
#define AKRYON_HAL_PCI_H

#include "types.h"

#define PCI_CONFIG_ADDRESS 0xCF8
#define PCI_CONFIG_DATA    0xCFC

typedef struct {
    uint8_t bus;
    uint8_t slot;
    uint8_t func;
    uint16_t vendor_id;
    uint16_t device_id;
    uint16_t class_id;
    uint16_t subclass_id;
    uint32_t bar0;
    uint8_t irq;
} pci_device_t;

uint32_t pci_read_config_32(uint8_t bus, uint8_t slot, uint8_t func, uint8_t offset);
uint16_t pci_read_config_16(uint8_t bus, uint8_t slot, uint8_t func, uint8_t offset);
uint8_t  pci_read_config_8(uint8_t bus, uint8_t slot, uint8_t func, uint8_t offset);
void     pci_write_config_32(uint8_t bus, uint8_t slot, uint8_t func, uint8_t offset, uint32_t val);
void     pci_write_config_16(uint8_t bus, uint8_t slot, uint8_t func, uint8_t offset, uint16_t val);

void pci_init(void);
int  pci_find_device(uint16_t vendor_id, uint16_t device_id, pci_device_t *out_dev);
void pci_enable_bus_master(const pci_device_t *dev);

#endif // AKRYON_HAL_PCI_H
