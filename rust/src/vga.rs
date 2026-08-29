use core::fmt::{self, Write};

extern "C" {
    fn vga_putchar(c: u8);
    fn vga_clear();
    fn vga_set_color(fg: u8, bg: u8);
    fn vga_backspace();
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black        = 0,
    Blue         = 1,
    Green        = 2,
    Cyan         = 3,
    Red          = 4,
    Magenta      = 5,
    Brown        = 6,
    LightGray    = 7,
    DarkGray     = 8,
    LightBlue    = 9,
    LightGreen   = 10,
    LightCyan    = 11,
    LightRed     = 12,
    LightMagenta = 13,
    Yellow       = 14,
    White        = 15,
}

impl Color {
    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => Color::Black,
            1 => Color::Blue,
            2 => Color::Green,
            3 => Color::Cyan,
            4 => Color::Red,
            5 => Color::Magenta,
            6 => Color::Brown,
            7 => Color::LightGray,
            8 => Color::DarkGray,
            9 => Color::LightBlue,
            10 => Color::LightGreen,
            11 => Color::LightCyan,
            12 => Color::LightRed,
            13 => Color::LightMagenta,
            14 => Color::Yellow,
            15 => Color::White,
            _ => Color::White,
        }
    }
}

pub struct VgaWriter;

impl Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            unsafe {
                vga_putchar(byte);
            }
        }
        Ok(())
    }
}

pub fn set_color(fg: Color, bg: Color) {
    unsafe {
        vga_set_color(fg as u8, bg as u8);
    }
}

pub fn clear_screen() {
    unsafe {
        vga_clear();
    }
}

pub fn backspace() {
    unsafe {
        vga_backspace();
    }
}

pub fn print_fmt(args: fmt::Arguments) {
    let mut writer = VgaWriter;
    let _ = writer.write_fmt(args);
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga::print_fmt(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => {{
        $crate::print!($($arg)*);
        $crate::print!("\n");
    }};
}

#[macro_export]
macro_rules! print_colored {
    ($fg:expr, $bg:expr, $($arg:tt)*) => {{
        $crate::vga::set_color($fg, $bg);
        $crate::print!($($arg)*);
        $crate::vga::set_color($crate::vga::Color::White, $crate::vga::Color::Black);
    }};
}
