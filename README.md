# Akryon OS ⚡

**Akryon OS** adalah sistem operasi hybrid modern yang dibangun dari awal (*from scratch*) untuk arsitektur **x86 (32-bit Protected Mode)** dengan menggabungkan keandalan **C & Assembly** pada level *Hardware Abstraction Layer (HAL)* dan keamanan memori serta kekuatan sistemik **Rust (`no_std`)** pada level *Kernel Core & Shell Subsystem*.

---

## 🌟 Arsitektur & Desain Sistem

```
                              +-----------------------------+
                              |        Akryon Shell         |
                              |  (Rust Interactive Console) |
                              +-----------------------------+
                                             |
                              +-----------------------------+
                              |      Rust Kernel Core       |
                              | (no_std, Format, Commands)  |
                              +-----------------------------+
                                             |  (FFI Bridge)
                              +-----------------------------+
                              |    C HAL & Device Drivers   |
                              | (GDT, IDT, PIC, PIT, Serial)|
                              +-----------------------------+
                                             |
                              +-----------------------------+
                              |    Assembly Glue & MBR      |
                              | (boot.asm, kernel_entry.asm)|
                              +-----------------------------+
                                             |
                              +-----------------------------+
                              |     Bare-Metal Hardware     |
                              +-----------------------------+
```

### 1. Bootloader & Low-Level Assembly (`boot/`)
- **`boot/boot.asm`**: MBR Bootloader (512 bytes) yang membaca kernel menggunakan BIOS LBA Extended Read (`int 0x13, AH=0x42`) dengan chunk buffer 64-sektor dan fallback CHS. Mengaktifkan Fast A20 Gate, memuat Flat 32-bit GDT, dan beralih ke 32-bit Protected Mode.
- **`boot/kernel_entry.asm`**: Entry point 32-bit mode (`0x10000`), inisialisasi stack di `0x90000`, mengaktifkan unit FPU/SSE (CR0/CR4), serta menyediakan assembly trampoline stubs untuk 32 Exception Vectors (ISR 0–31) dan 16 Hardware IRQ (IRQ 0–15).

### 2. Hardware Abstraction Layer / HAL (`hal/`)
- **`hal/io.h` & `hal/io.c`**: Primitif Port I/O x86 (`inb`, `outb`, `inw`, `outw`, `cli`, `sti`, `hlt`).
- **`hal/vga.h` & `hal/vga.c`**: Driver VGA text-mode 80x25 buffer (`0xB8000`), manajemen warna foreground & background, auto-scroll, dan update hardware cursor CRT controller (`0x3D4`/`0x3D5`).
- **`hal/gdt.h` & `hal/gdt.c`**: Global Descriptor Table dengan Kernel Code (`0x08`), Kernel Data (`0x10`), User Code (`0x18`), dan User Data (`0x20`).
- **`hal/idt.h`, `hal/idt.c`, `hal/isr.h`, `hal/isr.c`**: Interrupt Descriptor Table 256 gates, PIC 8259 Remapping (Master IRQ 0..7 $\rightarrow$ 32..39, Slave IRQ 8..15 $\rightarrow$ 40..47), serta CPU exception & IRQ dispatcher.
- **`hal/timer.h` & `hal/timer.c`**: PIT 8254 Channel 0 pada frekuensi 100Hz (10ms per tick), penghitung uptime dan fungsi delay/sleep.
- **`hal/keyboard.h` & `hal/keyboard.c`**: Driver Keyboard PS/2 berbasis interrupt (IRQ 1) dengan circular ring buffer, pemetaan Scancode Set 1 (US QWERTY), Shift modifier, CapsLock, dan keypad.
- **`hal/serial.h` & `hal/serial.c`**: Driver UART 16550 Serial Port (COM1 `0x3F8` @ 38400 baud) untuk kernel logging dan debugging.

### 3. Rust Core & Shell Subsystem (`rust/`)
- **`rust/src/lib.rs`**: `#![no_std]` Rust entry point (`akryon_rust_main`), banner boot ASCII, dan panic handler kustom berlatar belakang merah saat terjadi panic tak tertangani.
- **`rust/src/vga.rs`**: Safe VGA writer yang mengimplementasikan `core::fmt::Write`, menyediakan macro `print!`, `println!`, dan `print_colored!`.
- **`rust/src/serial.rs`**: Safe Serial logger yang mengimplementasikan macro `log!` dan `logln!`.
- **`rust/src/shell.rs`**: Interactive line editor dengan prompt `akryon> `, backspace handling, dan eksekusi perintah.
- **`rust/src/commands.rs`**: Perintah-perintah interaktif bawaan.

---

## 💻 Daftar Perintah Shell

| Perintah | Deskripsi |
|---|---|
| `help` | Menampilkan panduan dan daftar perintah yang tersedia |
| `clear` | Membersihkan layar dan menampilkan kembali banner Akryon |
| `about` | Menampilkan informasi arsitektur hybrid C & Rust OS |
| `sysinfo` | Menampilkan mode CPU, pointer stack, status interrupt, dan timer ticks |
| `uptime` | Menampilkan waktu aktif sistem sejak proses boot |
| `echo <text>` | Mencetak kembali teks yang diinput |
| `color <fg> <bg>` | Mengganti warna console secara dinamis (0..15) |
| `calc <a op b>` | Kalkulator aritmatika integer (contoh: `calc 42 + 58`, `calc 100 * 5`) |
| `panic [pesan]` | Memicu Rust Kernel Panic untuk demonstrasi crash handler |
| `reboot` | Melakukan soft reboot CPU |

---

## 📁 Struktur Direktori

```
Akryon/
├── boot/
│   ├── boot.asm             # MBR Bootloader (16-bit real mode -> 32-bit protected mode)
│   └── kernel_entry.asm     # 32-bit Entry point, FPU/SSE setup & ISR stubs
├── hal/
│   ├── types.h              # Freestanding primitive typedefs & memory prototypes
│   ├── string.c             # Implementasi freestanding memcpy, memset, memcmp, bcmp, strlen
│   ├── io.h / io.c          # Port I/O wrappers (inb, outb, inw, outw, cli, sti, hlt)
│   ├── vga.h / vga.c        # Driver VGA Text Console 80x25
│   ├── gdt.h / gdt.c        # Global Descriptor Table (GDT)
│   ├── idt.h / idt.c        # Interrupt Descriptor Table (IDT) & PIC 8259 Remap
│   ├── isr.h / isr.c        # Interrupt Service Routines & Exception handlers
│   ├── timer.h / timer.c    # PIT (Programmable Interval Timer) 100Hz
│   ├── keyboard.h / keyboard.c # Driver PS/2 Keyboard dengan ring buffer
│   ├── serial.h / serial.c  # Driver UART 16550 Serial COM1
│   └── hal.h                # Unified HAL Master Header
├── kernel/
│   └── kmain.c              # C Kernel initialization & Rust bridge
├── rust/
│   ├── Cargo.toml           # Konfigurasi Rust package
│   └── src/
│       ├── lib.rs           # Rust kernel entry, panic handler, banner
│       ├── vga.rs           # Safe VGA writer & print! macros
│       ├── serial.rs        # Safe Serial logger & log! macros
│       ├── shell.rs         # Interactive line-buffered shell
│       └── commands.rs      # Command interpreter engine
├── linker.ld                # Linker script (dimuat di 0x10000)
├── Makefile                 # Build system modular
└── README.md                # Dokumentasi proyek ini
```

---

## 🛠️ Prasyarat & Instalasi Toolchain

### Di Linux (Arch Linux / Debian / Ubuntu / Fedora)
- **NASM** (`nasm`)
- **GCC** (`gcc`)
- **GNU Binutils** (`ld`, `objcopy`)
- **Rust Toolchain** (`rustc`, `cargo` dengan target `i686-unknown-linux-gnu`)
- **QEMU** (`qemu-system-i386`)
- **Make** (`make`)

```bash
# Arch Linux
sudo pacman -S nasm gcc binutils qemu-system-x86 make rust

# Debian / Ubuntu
sudo apt install nasm gcc-multilib binutils qemu-system-x86 make rustc cargo

# Target Rust untuk 32-bit x86
rustup target add i686-unknown-linux-gnu
```

---

## 🚀 Cara Build & Menjalankan

### 1. Build OS Image
```bash
make clean
make
```
Perintah ini akan mengkompilasi bootloader NASM, HAL C, Rust static library, melakukan linking via `ld`, dan membuat file disk image `akryon.img`.

### 2. Menjalankan di QEMU
```bash
# Menjalankan dengan GUI Display QEMU
make run

# Menjalankan dengan output serial terhubung ke terminal stdio
make run-serial

# Menjalankan dalam mode Curses text console di terminal
make run-curses
```

### 3. Debugging dengan GDB
```bash
make debug
```
QEMU akan menunggu koneksi GDB pada port `localhost:1234`. Di terminal lain, jalankan:
```bash
gdb -ex "target remote localhost:1234" -ex "symbol-file build/kernel.elf"
```

---

## 📜 Lisensi
Bebas digunakan, dimodifikasi, dan dikembangkan untuk keperluan edukasi dan eksplorasi pengembangan sistem operasi.
