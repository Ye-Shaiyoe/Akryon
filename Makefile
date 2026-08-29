# ==============================================================================
# Akryon OS - Makefile
# Hybrid C and Rust Operating System Build System
# ==============================================================================

export PATH := /home/akrom/.cargo/bin:/home/akrom/.local/bin:$(PATH)

# Toolchain
ASM      := nasm
CC       := gcc
LD       := ld
OBJCOPY  := objcopy
RUSTC    := rustc

# Directories
BUILD_DIR := build
BOOT_DIR  := boot
HAL_DIR   := hal
KERN_DIR  := kernel
RUST_DIR  := rust

# Target Files
OS_IMAGE   := akryon.img
BOOT_BIN   := $(BUILD_DIR)/boot.bin
KERNEL_BIN := $(BUILD_DIR)/kernel.bin
KERNEL_ELF := $(BUILD_DIR)/kernel.elf
RUST_LIB   := $(BUILD_DIR)/libakryon_rust.a

# Flags
ASM_FLAGS  := -f elf32
C_FLAGS    := -m32 -mno-sse -mno-mmx -mno-sse2 -ffreestanding -fno-pie \
              -fno-stack-protector -fno-builtin -nostdlib -nostdinc \
              -Wall -Wextra -O2 -I$(HAL_DIR) -c
RUST_TARGET:= i686-unknown-linux-gnu
RUST_FLAGS := --target $(RUST_TARGET) --crate-type staticlib -C panic=abort \
              -C relocation-model=static -C opt-level=2
LD_FLAGS   := -m elf_i386 -T linker.ld -nostdlib

# C Objects
C_OBJS := $(BUILD_DIR)/string.o \
          $(BUILD_DIR)/io.o \
          $(BUILD_DIR)/vga.o \
          $(BUILD_DIR)/gdt.o \
          $(BUILD_DIR)/idt.o \
          $(BUILD_DIR)/isr.o \
          $(BUILD_DIR)/timer.o \
          $(BUILD_DIR)/keyboard.o \
          $(BUILD_DIR)/serial.o \
          $(BUILD_DIR)/kmain.o

# Rust Source Files
RUST_SRCS := $(shell find $(RUST_DIR)/src -name '*.rs')

# Default Target
all: $(OS_IMAGE)

# Ensure build directory exists
$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

# 1. Build MBR Bootloader (16-bit binary)
$(BOOT_BIN): $(BOOT_DIR)/boot.asm | $(BUILD_DIR)
	$(ASM) -f bin $< -o $@

# 2. Build Assembly Kernel Entry and ISR Trampolines
$(BUILD_DIR)/kernel_entry.o: $(BOOT_DIR)/kernel_entry.asm | $(BUILD_DIR)
	$(ASM) $(ASM_FLAGS) $< -o $@

# 3. Build C HAL Driver Objects
$(BUILD_DIR)/string.o: $(HAL_DIR)/string.c $(HAL_DIR)/types.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/io.o: $(HAL_DIR)/io.c $(HAL_DIR)/io.h $(HAL_DIR)/types.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@
$(BUILD_DIR)/vga.o: $(HAL_DIR)/vga.c $(HAL_DIR)/vga.h $(HAL_DIR)/io.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/gdt.o: $(HAL_DIR)/gdt.c $(HAL_DIR)/gdt.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/idt.o: $(HAL_DIR)/idt.c $(HAL_DIR)/idt.h $(HAL_DIR)/io.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/isr.o: $(HAL_DIR)/isr.c $(HAL_DIR)/isr.h $(HAL_DIR)/vga.h $(HAL_DIR)/io.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/timer.o: $(HAL_DIR)/timer.c $(HAL_DIR)/timer.h $(HAL_DIR)/isr.h $(HAL_DIR)/io.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/keyboard.o: $(HAL_DIR)/keyboard.c $(HAL_DIR)/keyboard.h $(HAL_DIR)/isr.h $(HAL_DIR)/io.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

$(BUILD_DIR)/serial.o: $(HAL_DIR)/serial.c $(HAL_DIR)/serial.h $(HAL_DIR)/io.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

# 4. Build C Kernel Main
$(BUILD_DIR)/kmain.o: $(KERN_DIR)/kmain.c $(HAL_DIR)/hal.h | $(BUILD_DIR)
	$(CC) $(C_FLAGS) $< -o $@

# 5. Build Rust Static Library
$(RUST_LIB): $(RUST_SRCS) $(RUST_DIR)/Cargo.toml | $(BUILD_DIR)
	$(RUSTC) $(RUST_FLAGS) $(RUST_DIR)/src/lib.rs -o $@

# 6. Link Assembly, C HAL, and Rust staticlib into Kernel ELF
$(KERNEL_ELF): $(BUILD_DIR)/kernel_entry.o $(C_OBJS) $(RUST_LIB) linker.ld
	$(LD) $(LD_FLAGS) $(BUILD_DIR)/kernel_entry.o $(C_OBJS) $(RUST_LIB) -o $@

# 7. Convert Kernel ELF to Raw Flat Binary
$(KERNEL_BIN): $(KERNEL_ELF)
	$(OBJCOPY) -O binary $< $@

# 8. Create Floppy Disk Image (1.44MB)
$(OS_IMAGE): $(BOOT_BIN) $(KERNEL_BIN)
	cat $(BOOT_BIN) $(KERNEL_BIN) > $(OS_IMAGE)
	truncate -s 1474560 $(OS_IMAGE)
	@echo "\n>>> Akryon OS Image successfully built: $(OS_IMAGE) (1.44 MB) <<<\n"

# Run in QEMU (GUI)
run: $(OS_IMAGE)
	qemu-system-i386 -drive file=$(OS_IMAGE),format=raw

# Run in QEMU with Serial output directed to terminal stdio
run-serial: $(OS_IMAGE)
	qemu-system-i386 -drive file=$(OS_IMAGE),format=raw -serial stdio

# Run in QEMU with Curses text console mode
run-curses: $(OS_IMAGE)
	qemu-system-i386 -drive file=$(OS_IMAGE),format=raw -curses

# Run in QEMU with GDB Debug Server (waiting on port 1234)
debug: $(OS_IMAGE)
	qemu-system-i386 -drive file=$(OS_IMAGE),format=raw -s -S -serial stdio

# Clean build artifacts
clean:
	rm -rf $(BUILD_DIR) $(OS_IMAGE) *.bin *.o akromos.img

.PHONY: all run run-serial run-curses debug clean