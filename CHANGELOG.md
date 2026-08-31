# Changelog

All notable changes to **Akryon OS** will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added
- Multi-language Kernel Architecture (Assembly + C HAL + Rust Shell/Core).
- Real-mode 16-bit MBR Bootloader (`boot.asm`) loading kernel into Protected Mode 32-bit.
- Hardware Abstraction Layer (HAL):
  - GDT (Global Descriptor Table) setup.
  - IDT (Interrupt Descriptor Table) and ISR handlers.
  - 8259 PIC remapping.
  - Programmable Interval Timer (PIT) IRQ0 timer driver.
  - PS/2 Keyboard IRQ1 driver with scan-code decoding.
  - 16550 UART Serial driver for debugging.
  - VGA 80x25 Color text driver with scrolling.
- Rust Core Subsystem:
  - Interactive Shell / CLI with commands (`help`, `clear`, `about`, `echo`, `reboot`, etc.).
  - Zero-allocation C-compatible string and buffer helpers.
- Automated CI Build Workflow with GitHub Actions.
