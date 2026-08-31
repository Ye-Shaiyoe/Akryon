use crate::vga::{self, Color};
use crate::commands;
use crate::{print_colored, println};

extern "C" {
    fn keyboard_getchar() -> u16;
}

// Special extended keycodes matching hal/keyboard.h
pub const KEY_UP: u16 = 0x0100;
pub const KEY_DOWN: u16 = 0x0101;
pub const KEY_LEFT: u16 = 0x0102;
pub const KEY_RIGHT: u16 = 0x0103;
pub const KEY_HOME: u16 = 0x0104;
pub const KEY_END: u16 = 0x0105;
pub const KEY_PAGE_UP: u16 = 0x0106;
pub const KEY_PAGE_DOWN: u16 = 0x0107;
pub const KEY_INSERT: u16 = 0x0108;
pub const KEY_DELETE: u16 = 0x0109;

// Control character constants
pub const KEY_CTRL_A: u16 = 0x0001; // Beginning of line (Home)
pub const KEY_CTRL_C: u16 = 0x0003; // Cancel / Interrupt (^C)
pub const KEY_CTRL_D: u16 = 0x0004; // Delete char at cursor
pub const KEY_CTRL_E: u16 = 0x0005; // End of line (End)
pub const KEY_BACKSPACE: u16 = 0x0008; // Backspace (\b)
pub const KEY_TAB: u16 = 0x0009; // Tab (\t)
pub const KEY_ENTER: u16 = 0x000A; // Enter (\n)
pub const KEY_CTRL_K: u16 = 0x000B; // Kill from cursor to end of line
pub const KEY_CTRL_L: u16 = 0x000C; // Clear screen (^L)
pub const KEY_RETURN: u16 = 0x000D; // Carriage Return (\r)
pub const KEY_CTRL_U: u16 = 0x0015; // Kill entire line
pub const KEY_CTRL_W: u16 = 0x0017; // Delete previous word
pub const KEY_DEL_CHAR: u16 = 0x007F; // ASCII Del


const HISTORY_CAPACITY: usize = 16;
const MAX_CMD_LEN: usize = 70;

struct CommandHistory {
    entries: [[u8; MAX_CMD_LEN]; HISTORY_CAPACITY],
    lens: [usize; HISTORY_CAPACITY],
    count: usize,
}

impl CommandHistory {
    const fn new() -> Self {
        Self {
            entries: [[0; MAX_CMD_LEN]; HISTORY_CAPACITY],
            lens: [0; HISTORY_CAPACITY],
            count: 0,
        }
    }

    fn push(&mut self, cmd: &[u8]) {
        if cmd.is_empty() {
            return;
        }
        // Avoid duplicate consecutive history entries
        if self.count > 0 {
            let last_len = self.lens[self.count - 1];
            if last_len == cmd.len() && &self.entries[self.count - 1][..last_len] == cmd {
                return;
            }
        }

        if self.count < HISTORY_CAPACITY {
            let idx = self.count;
            self.entries[idx][..cmd.len()].copy_from_slice(cmd);
            self.lens[idx] = cmd.len();
            self.count += 1;
        } else {
            // Shift history entries when buffer is full
            for i in 0..HISTORY_CAPACITY - 1 {
                self.entries[i] = self.entries[i + 1];
                self.lens[i] = self.lens[i + 1];
            }
            let idx = HISTORY_CAPACITY - 1;
            self.entries[idx][..cmd.len()].copy_from_slice(cmd);
            self.lens[idx] = cmd.len();
        }
    }

    fn get(&self, idx: usize) -> Option<&[u8]> {
        if idx < self.count {
            Some(&self.entries[idx][..self.lens[idx]])
        } else {
            None
        }
    }
}

pub fn run_shell() -> ! {
    let mut history = CommandHistory::new();
    let mut buffer = [0u8; MAX_CMD_LEN];
    let mut len: usize = 0;
    let mut cursor: usize = 0;

    let mut draft_buffer = [0u8; MAX_CMD_LEN];
    let mut draft_len: usize = 0;
    let mut hist_index: Option<usize> = None;

    print_prompt();
    let (mut prompt_x, mut prompt_y) = vga::get_cursor();

    loop {
        let key = unsafe { keyboard_getchar() };

        match key {
            // Enter key: execute command
            KEY_ENTER | KEY_RETURN => {
                vga::set_cursor(prompt_x + len, prompt_y);
                println!();

                if len > 0 {
                    let cmd_slice = &buffer[..len];
                    history.push(cmd_slice);
                    if let Ok(cmd_str) = core::str::from_utf8(cmd_slice) {
                        commands::handle_command(cmd_str);
                    }
                    len = 0;
                    cursor = 0;
                }

                hist_index = None;
                print_prompt();
                let (nx, ny) = vga::get_cursor();
                prompt_x = nx;
                prompt_y = ny;
            }


            // Ctrl + C: Cancel current line & start new prompt
            KEY_CTRL_C => {
                vga::set_cursor(prompt_x + len, prompt_y);
                print_colored!(Color::LightRed, Color::Black, "^C\n");
                len = 0;
                cursor = 0;
                hist_index = None;
                print_prompt();
                let (nx, ny) = vga::get_cursor();
                prompt_x = nx;
                prompt_y = ny;
            }

            // Ctrl + L: Clear screen and restore prompt & current line buffer
            KEY_CTRL_L => {
                vga::clear_screen();
                print_prompt();
                let (nx, ny) = vga::get_cursor();
                prompt_x = nx;
                prompt_y = ny;
                redraw_line(prompt_x, prompt_y, &buffer, len, cursor, 0);
            }

            // Left Arrow: Move cursor left
            KEY_LEFT => {
                if cursor > 0 {
                    cursor -= 1;
                    vga::set_cursor(prompt_x + cursor, prompt_y);
                }
            }

            // Right Arrow: Move cursor right
            KEY_RIGHT => {
                if cursor < len {
                    cursor += 1;
                    vga::set_cursor(prompt_x + cursor, prompt_y);
                }
            }

            // Home / Ctrl+A: Jump to beginning of line
            KEY_HOME | KEY_CTRL_A => {
                cursor = 0;
                vga::set_cursor(prompt_x, prompt_y);
            }

            // End / Ctrl+E: Jump to end of line
            KEY_END | KEY_CTRL_E => {
                cursor = len;
                vga::set_cursor(prompt_x + len, prompt_y);
            }

            // Up Arrow: Navigate command history (previous)
            KEY_UP => {
                if history.count > 0 {
                    let next_idx = match hist_index {
                        None => {
                            // Save current draft
                            draft_buffer[..len].copy_from_slice(&buffer[..len]);
                            draft_len = len;
                            history.count - 1
                        }
                        Some(i) if i > 0 => i - 1,
                        Some(i) => i,
                    };

                    hist_index = Some(next_idx);
                    if let Some(entry) = history.get(next_idx) {
                        let old_len = len;
                        buffer[..entry.len()].copy_from_slice(entry);
                        len = entry.len();
                        cursor = len;
                        redraw_line(prompt_x, prompt_y, &buffer, len, cursor, old_len);
                    }
                }
            }

            // Down Arrow: Navigate command history (next)
            KEY_DOWN => {
                if let Some(idx) = hist_index {
                    if idx + 1 < history.count {
                        let next_idx = idx + 1;
                        hist_index = Some(next_idx);
                        if let Some(entry) = history.get(next_idx) {
                            let old_len = len;
                            buffer[..entry.len()].copy_from_slice(entry);
                            len = entry.len();
                            cursor = len;
                            redraw_line(prompt_x, prompt_y, &buffer, len, cursor, old_len);
                        }
                    } else {
                        // Restore draft buffer
                        hist_index = None;
                        let old_len = len;
                        buffer[..draft_len].copy_from_slice(&draft_buffer[..draft_len]);
                        len = draft_len;
                        cursor = len;
                        redraw_line(prompt_x, prompt_y, &buffer, len, cursor, old_len);
                    }
                }
            }

            // Backspace: Delete character before cursor
            KEY_BACKSPACE | KEY_DEL_CHAR => {

                if cursor > 0 {
                    let old_len = len;
                    for i in cursor..len {
                        buffer[i - 1] = buffer[i];
                    }
                    cursor -= 1;
                    len -= 1;
                    redraw_line(prompt_x, prompt_y, &buffer, len, cursor, old_len);
                }
            }

            // Delete / Ctrl+D: Delete character at cursor
            KEY_DELETE | KEY_CTRL_D => {
                if cursor < len {
                    let old_len = len;
                    for i in (cursor + 1)..len {
                        buffer[i - 1] = buffer[i];
                    }
                    len -= 1;
                    redraw_line(prompt_x, prompt_y, &buffer, len, cursor, old_len);
                }
            }

            // Ctrl + U: Clear entire line
            KEY_CTRL_U => {
                if len > 0 {
                    let old_len = len;
                    len = 0;
                    cursor = 0;
                    redraw_line(prompt_x, prompt_y, &buffer, len, cursor, old_len);
                }
            }

            // Ctrl + K: Kill line from cursor to end
            KEY_CTRL_K => {
                if cursor < len {
                    let old_len = len;
                    len = cursor;
                    redraw_line(prompt_x, prompt_y, &buffer, len, cursor, old_len);
                }
            }

            // Ctrl + W: Delete word backwards
            KEY_CTRL_W => {
                if cursor > 0 {
                    let old_len = len;
                    let mut new_cursor = cursor;
                    // Skip spaces before cursor
                    while new_cursor > 0 && buffer[new_cursor - 1] == b' ' {
                        new_cursor -= 1;
                    }
                    // Skip word characters
                    while new_cursor > 0 && buffer[new_cursor - 1] != b' ' {
                        new_cursor -= 1;
                    }
                    let deleted_count = cursor - new_cursor;
                    for i in cursor..len {
                        buffer[i - deleted_count] = buffer[i];
                    }
                    len -= deleted_count;
                    cursor = new_cursor;
                    redraw_line(prompt_x, prompt_y, &buffer, len, cursor, old_len);
                }
            }

            // Printable ASCII characters
            ascii if ascii >= 32 && ascii <= 126 => {
                let ch = ascii as u8;
                if len < MAX_CMD_LEN - 1 {
                    let old_len = len;
                    // Shift right for insertion
                    for i in (cursor..len).rev() {
                        buffer[i + 1] = buffer[i];
                    }
                    buffer[cursor] = ch;
                    cursor += 1;
                    len += 1;
                    redraw_line(prompt_x, prompt_y, &buffer, len, cursor, old_len);
                }
            }

            // Ignore unhandled keys
            _ => {}
        }
    }
}

fn redraw_line(
    prompt_x: usize,
    prompt_y: usize,
    buffer: &[u8],
    len: usize,
    cursor: usize,
    old_len: usize,
) {
    vga::set_cursor(prompt_x, prompt_y);
    for i in 0..len {
        vga::putchar(buffer[i]);
    }
    // Clear any extra characters if previous line was longer
    if old_len > len {
        for _ in len..old_len {
            vga::putchar(b' ');
        }
    }
    vga::set_cursor(prompt_x + cursor, prompt_y);
}

fn print_prompt() {
    print_colored!(Color::LightGreen, Color::Black, "akryon");
    print_colored!(Color::LightCyan, Color::Black, "> ");
}

