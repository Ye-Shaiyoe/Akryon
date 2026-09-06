#include "pci.h"
#include "io.h"
#include "serial.h"

static uint32_t pci_get_address(uint8_t bus, uint8_t slot, uint8_t func, uint8_t offset) {
    return (1U << 31)
         | ((uint32_t)bus << 16)
         | ((uint32_t)(slot & 0x1F) << 11)
         | ((uint32_t)(func & 0x07) << 8)
         | ((uint32_t)(offset & 0xFC));
}

uint32_t pci_read_config_32(uint8_t bus, uint8_t slot, uint8_t func, uint8_t offset) {
    outl(PCI_CONFIG_ADDRESS, pci_get_address(bus, slot, func, offset));
    return inl(PCI_CONFIG_DATA);
}

uint16_t pci_read_config_16(uint8_t bus, uint8_t slot, uint8_t func, uint8_t offset) {
    uint32_t val = pci_read_config_32(bus, slot, func, offset);
    return (uint16_t)((val >> ((offset & 2) * 8)) & 0xFFFF);
}

uint8_t pci_read_config_8(uint8_t bus, uint8_t slot, uint8_t func, uint8_t offset) {
    uint32_t val = pci_read_config_32(bus, slot, func, offset);
    return (uint8_t)((val >> ((offset & 3) * 8)) & 0xFF);
}

void pci_write_config_32(uint8_t bus, uint8_t slot, uint8_t func, uint8_t offset, uint32_t val) {
    outl(PCI_CONFIG_ADDRESS, pci_get_address(bus, slot, func, offset));
    outl(PCI_CONFIG_DATA, val);
}

void pci_write_config_16(uint8_t bus, uint8_t slot, uint8_t func, uint8_t offset, uint16_t val) {
    uint32_t old_val = pci_read_config_32(bus, slot, func, offset);
    uint32_t shift = (offset & 2) * 8;
    uint32_t mask = 0xFFFFU << shift;
    uint32_t new_val = (old_val & ~mask) | (((uint32_t)val) << shift);
    pci_write_config_32(bus, slot, func, offset, new_val);
}

void pci_enable_bus_master(const pci_device_t *dev) {
    uint16_t cmd = pci_read_config_16(dev->bus, dev->slot, dev->func, 0x04);
    cmd |= (1 << 0) | (1 << 2); // Enable I/O Space (bit 0) and Bus Mastering (bit 2)
    pci_write_config_16(dev->bus, dev->slot, dev->func, 0x04, cmd);
}

static int pci_probe_device(uint8_t bus, uint8_t slot, uint8_t func, pci_device_t *out_dev) {
    uint16_t vendor_id = pci_read_config_16(bus, slot, func, 0x00);
    if (vendor_id == 0xFFFF || vendor_id == 0x0000) {
        return 0;
    }

    out_dev->bus = bus;
    out_dev->slot = slot;
    out_dev->func = func;
    out_dev->vendor_id = vendor_id;
    out_dev->device_id = pci_read_config_16(bus, slot, func, 0x02);
    out_dev->subclass_id = pci_read_config_8(bus, slot, func, 0x0A);
    out_dev->class_id = pci_read_config_8(bus, slot, func, 0x0B);
    out_dev->bar0 = pci_read_config_32(bus, slot, func, 0x10);
    out_dev->irq = pci_read_config_8(bus, slot, func, 0x3C);
    return 1;
}

int pci_find_device(uint16_t vendor_id, uint16_t device_id, pci_device_t *out_dev) {
    for (uint16_t bus = 0; bus < 256; bus++) {
        for (uint8_t slot = 0; slot < 32; slot++) {
            pci_device_t dev;
            if (!pci_probe_device((uint8_t)bus, slot, 0, &dev)) {
                continue;
            }

            uint8_t header_type = pci_read_config_8((uint8_t)bus, slot, 0, 0x0E);
            uint8_t max_func = (header_type & 0x80) ? 8 : 1;

            for (uint8_t func = 0; func < max_func; func++) {
                if (pci_probe_device((uint8_t)bus, slot, func, &dev)) {
                    if (dev.vendor_id == vendor_id && dev.device_id == device_id) {
                        *out_dev = dev;
                        return 1;
                    }
                }
            }
        }
    }
    return 0;
}

void pci_init(void) {
    serial_puts("[Akryon HAL] Scanning PCI Bus...\n");

    for (uint16_t bus = 0; bus < 256; bus++) {
        for (uint8_t slot = 0; slot < 32; slot++) {
            pci_device_t dev;
            if (!pci_probe_device((uint8_t)bus, slot, 0, &dev)) {
                continue;
            }

            uint8_t header_type = pci_read_config_8((uint8_t)bus, slot, 0, 0x0E);
            uint8_t max_func = (header_type & 0x80) ? 8 : 1;

            for (uint8_t func = 0; func < max_func; func++) {
                if (pci_probe_device((uint8_t)bus, slot, func, &dev)) {
                    serial_puts("  PCI: [");
                    serial_puthex8(dev.bus);
                    serial_puts(":");
                    serial_puthex8(dev.slot);
                    serial_puts(".");
                    serial_putdec(dev.func);
                    serial_puts("] Vendor=");
                    serial_puthex16(dev.vendor_id);
                    serial_puts(" Device=");
                    serial_puthex16(dev.device_id);
                    serial_puts(" IRQ=");
                    serial_putdec(dev.irq);
                    serial_puts("\n");
                }
            }
        }
    }
}
