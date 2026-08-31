# Contributing to Akryon OS

Terima kasih atas ketertarikan Anda untuk berkontribusi pada pengembangan **Akryon OS**! 🚀

---

## 🛠️ Persyaratan Lingkungan (Toolchain)

Sebelum berkontribusi, pastikan toolchain berikut telah terpasang di sistem Anda:

- **NASM** (Assembler x86): `sudo apt install nasm`
- **GCC Multilib** (32-bit compilation): `sudo apt install gcc-multilib build-essential`
- **Rust Toolchain**: `rustup` dengan target 32-bit:
  ```bash
  rustup target add i686-unknown-linux-gnu
  ```
- **QEMU x86**: `sudo apt install qemu-system-x86`

---

## 📋 Alur Kontribusi (Workflow)

1. **Fork & Clone** repositori ini.
2. Buat branch fitur/perbaikan baru:
   ```bash
   git checkout -b feature/nama-fitur
   ```
3. Lakukan pengujian kompilasi dan runtime:
   ```bash
   make clean
   make
   make run
   ```
4. Pastikan tidak ada compiler warning atau memory faults saat runtime.
5. Commit perubahan dengan pesan yang jelas (mengikuti konvensi [Conventional Commits](https://www.conventionalcommits.org/)):
   - `feat: add AHCI SATA driver`
   - `fix: fix keyboard ISR race condition`
   - `docs: update build instructions in README`
6. Push ke branch Anda dan buat **Pull Request (PR)** ke branch `main`.

---

## 🎨 Gaya Penulisan Kode (Code Style)

- **C & Assembly**: Gunakan indentasi 4 spasi. Ikuti format yang ditentukan di `.clang-format`.
- **Rust**: Ikuti standar `rustfmt` (`cargo fmt`). Selalu gunakan anotasi `#[no_mangle]` dan `pub extern "C"` untuk fungsi FFI C-Rust.
