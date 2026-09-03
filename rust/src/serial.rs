use core::fmt::{self, Write};

extern "C" {
    fn serial_putchar(c: u8);
    fn serial_puts(s: *const u8);
    fn serial_putdec(val: u32);
}

pub struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            if byte == b'\n' {
                unsafe {
                    serial_putchar(b'\r');
                }
            }
            unsafe {
                serial_putchar(byte);
            }
        }
        Ok(())
    }
}

pub fn log_fmt(args: fmt::Arguments) {
    let mut writer = SerialWriter;
    let _ = writer.write_fmt(args);
}

pub fn log_mem(total_kb: usize, free_kb: usize) {
    unsafe {
        serial_puts(b"[Akryon Kernel] Memory: total \0".as_ptr());
        serial_putdec(total_kb as u32);
        serial_puts(b" KB, free \0".as_ptr());
        serial_putdec(free_kb as u32);
        serial_puts(b" KB\n\0".as_ptr());
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => ($crate::serial::log_fmt(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! logln {
    () => ($crate::log!("\n"));
    ($($arg:tt)*) => {{
        $crate::log!($($arg)*);
        $crate::log!("\n");
    }};
}
