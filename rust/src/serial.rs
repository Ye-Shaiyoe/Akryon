use core::fmt::{self, Write};

extern "C" {
    fn serial_putchar(c: u8);
}

pub struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
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
