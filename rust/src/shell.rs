use crate::vga::{self, Color};
use crate::commands;
use crate::{print, println, print_colored};

extern "C" {
    fn keyboard_getchar() -> u8;
}

pub fn run_shell() -> ! {
    let mut buffer = [0u8; 256];
    let mut cursor: usize = 0;

    print_prompt();

    loop {
        let ch = unsafe { keyboard_getchar() };

        if ch == b'\n' || ch == b'\r' {
            println!();
            if cursor > 0 {
                if let Ok(cmd_str) = core::str::from_utf8(&buffer[..cursor]) {
                    commands::handle_command(cmd_str);
                }
                cursor = 0;
            }
            print_prompt();
        } else if ch == b'\x08' || ch == 0x7F {
            if cursor > 0 {
                cursor -= 1;
                vga::backspace();
            }
        } else if ch >= 32 && ch <= 126 {
            if cursor < buffer.len() - 1 {
                buffer[cursor] = ch;
                cursor += 1;
                print!("{}", ch as char);
            }
        }
    }
}

fn print_prompt() {
    print_colored!(Color::LightGreen, Color::Black, "akryon");
    print_colored!(Color::LightCyan, Color::Black, "> ");
}
