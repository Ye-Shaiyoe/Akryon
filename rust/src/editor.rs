use crate::vga::{self, Color};
use crate::shell::{
    KEY_UP, KEY_DOWN, KEY_LEFT, KEY_RIGHT,
    KEY_HOME, KEY_END, KEY_PAGE_UP, KEY_PAGE_DOWN,
    KEY_BACKSPACE, KEY_DEL_CHAR, KEY_DELETE,
    KEY_ENTER, KEY_RETURN,
    KEY_CTRL_S, KEY_CTRL_Q,
    KEY_CTRL_A, KEY_CTRL_E, KEY_CTRL_K,
};
use alloc::vec::Vec;
use alloc::string::String;

const EDITOR_ROWS: usize = 23;  // VGA rows available for text (0..22)
const EDITOR_COLS: usize = 80;  // VGA width
const STATUS_ROW: usize = 24;   // Bottom status bar row (VGA row 24)
const MAX_LINES: usize = 512;   // Maximum buffer lines

extern "C" {
    fn keyboard_getchar() -> u16;
    fn vga_putchar_at(c: u8, color: u8, x: usize, y: usize);
}

/// Core text editor state.
pub struct Editor {
    /// Text buffer: each entry is a line (Vec<u8> of ASCII bytes)
    lines: Vec<Vec<u8>>,
    /// Cursor row within the buffer (0-indexed)
    cursor_row: usize,
    /// Cursor column within the current line (0-indexed)
    cursor_col: usize,
    /// First visible row offset (for vertical scrolling)
    scroll_row: usize,
    /// File name in VFS
    filename: String,
    /// Whether the buffer has been modified since last save
    modified: bool,
}

impl Editor {
    /// Create a new editor instance for the given filename and initial content.
    fn new(filename: &str, content: &[u8]) -> Self {
        let mut lines: Vec<Vec<u8>> = Vec::new();

        if content.is_empty() {
            lines.push(Vec::new());
        } else {
            let mut current_line: Vec<u8> = Vec::new();
            for &b in content {
                if b == b'\n' {
                    lines.push(current_line);
                    current_line = Vec::new();
                } else if b != b'\r' {
                    current_line.push(b);
                }
            }
            // Push last line even if no trailing newline
            if !current_line.is_empty() || content.last() != Some(&b'\n') {
                lines.push(current_line);
            }
        }

        if lines.is_empty() {
            lines.push(Vec::new());
        }

        Editor {
            lines,
            cursor_row: 0,
            cursor_col: 0,
            scroll_row: 0,
            filename: String::from(filename),
            modified: false,
        }
    }

    // -----------------------------------------------------------------------
    // Rendering
    // -----------------------------------------------------------------------

    /// Render the full editor (text area + status bar).
    fn render_all(&self) {
        self.render_text_area();
        self.render_status_bar();
        self.update_hw_cursor();
    }

    /// Render all visible text rows.
    fn render_text_area(&self) {
        for screen_row in 0..EDITOR_ROWS {
            self.render_row(screen_row);
        }
    }

    /// Render a single screen row.
    fn render_row(&self, screen_row: usize) {
        let buf_row = self.scroll_row + screen_row;
        let line = if buf_row < self.lines.len() {
            &self.lines[buf_row]
        } else {
            // Empty row past end of file - render tilde like vim
            let color = vga::make_color(Color::DarkGray, Color::Black);
            unsafe { vga_putchar_at(b'~', color, 0, screen_row); }
            // Clear rest of row
            let bg = vga::make_color(Color::LightGray, Color::Black);
            for col in 1..EDITOR_COLS {
                unsafe { vga_putchar_at(b' ', bg, col, screen_row); }
            }
            return;
        };

        let text_color = vga::make_color(Color::White, Color::Black);
        let bg_color = vga::make_color(Color::LightGray, Color::Black);

        for col in 0..EDITOR_COLS {
            let ch = if col < line.len() { line[col] } else { b' ' };
            let color = if col < line.len() { text_color } else { bg_color };
            unsafe { vga_putchar_at(ch, color, col, screen_row); }
        }
    }

    /// Render the bottom status bar (row 24).
    fn render_status_bar(&self) {
        let bar_color = vga::make_color(Color::Black, Color::LightGray);
        let hint_color = vga::make_color(Color::DarkGray, Color::LightGray);
        let mod_color = vga::make_color(Color::Red, Color::LightGray);

        // Build left part: filename + [Modified]
        let mut col = 0;

        // "mway - " prefix
        let prefix = b"mway - ";
        for &b in prefix {
            unsafe { vga_putchar_at(b, bar_color, col, STATUS_ROW - 1); }
            col += 1;
        }

        // Filename
        for b in self.filename.bytes() {
            if col >= 40 { break; }
            unsafe { vga_putchar_at(b, bar_color, col, STATUS_ROW - 1); }
            col += 1;
        }

        // [Modified] marker
        if self.modified {
            let marker = b" [Modified]";
            for &b in marker {
                if col >= 50 { break; }
                let c = if b == b'[' || b == b']' { bar_color } else { mod_color };
                unsafe { vga_putchar_at(b, c, col, STATUS_ROW - 1); }
                col += 1;
            }
        }

        // Fill middle with spaces
        while col < 55 {
            unsafe { vga_putchar_at(b' ', bar_color, col, STATUS_ROW - 1); }
            col += 1;
        }

        // Right part: position info + shortcuts
        let total_lines = self.lines.len();
        let row_display = self.cursor_row + 1;
        let col_display = self.cursor_col + 1;

        // Position: "Ln XX/XX Col XX"
        // Build right section string manually (no format!, it allocates)
        let hint = b"^S Save  ^Q Exit  ";
        // Print hint first at the right side
        let hint_start = EDITOR_COLS - hint.len();
        for (i, &b) in hint.iter().enumerate() {
            unsafe { vga_putchar_at(b, hint_color, hint_start + i, STATUS_ROW - 1); }
        }

        // Print line/col info
        let pos_start = 55;
        // "Ln:"
        let ln_label = b"Ln:";
        let mut pcol = pos_start;
        for &b in ln_label {
            unsafe { vga_putchar_at(b, bar_color, pcol, STATUS_ROW - 1); }
            pcol += 1;
        }
        pcol = write_number_at(row_display, pcol, STATUS_ROW - 1, bar_color);
        unsafe { vga_putchar_at(b'/', bar_color, pcol, STATUS_ROW - 1); }
        pcol += 1;
        pcol = write_number_at(total_lines, pcol, STATUS_ROW - 1, bar_color);
        unsafe { vga_putchar_at(b' ', bar_color, pcol, STATUS_ROW - 1); }
        pcol += 1;
        // "Col:"
        let col_label = b"Col:";
        for &b in col_label {
            if pcol < hint_start { unsafe { vga_putchar_at(b, bar_color, pcol, STATUS_ROW - 1); } }
            pcol += 1;
        }
        let _ = write_number_at(col_display, pcol, STATUS_ROW - 1, bar_color);
    }

    /// Move hardware cursor to match logical cursor position.
    fn update_hw_cursor(&self) {
        let screen_row = self.cursor_row.saturating_sub(self.scroll_row);
        let screen_col = self.cursor_col.min(EDITOR_COLS - 1);
        vga::set_cursor(screen_col, screen_row);
    }

    // -----------------------------------------------------------------------
    // Scrolling
    // -----------------------------------------------------------------------

    /// Ensure the cursor row is visible; adjust scroll_row if needed.
    fn scroll_to_cursor(&mut self) {
        if self.cursor_row < self.scroll_row {
            self.scroll_row = self.cursor_row;
        } else if self.cursor_row >= self.scroll_row + EDITOR_ROWS {
            self.scroll_row = self.cursor_row - EDITOR_ROWS + 1;
        }
    }

    // -----------------------------------------------------------------------
    // Cursor movement
    // -----------------------------------------------------------------------

    fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            let line_len = self.lines[self.cursor_row].len();
            if self.cursor_col > line_len {
                self.cursor_col = line_len;
            }
        }
    }

    fn move_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            let line_len = self.lines[self.cursor_row].len();
            if self.cursor_col > line_len {
                self.cursor_col = line_len;
            }
        }
    }

    fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            // Wrap to end of previous line
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
        }
    }

    fn move_right(&mut self) {
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.lines.len() {
            // Wrap to beginning of next line
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    fn move_home(&mut self) {
        self.cursor_col = 0;
    }

    fn move_end(&mut self) {
        self.cursor_col = self.lines[self.cursor_row].len();
    }

    fn page_up(&mut self) {
        let rows = EDITOR_ROWS;
        if self.cursor_row >= rows {
            self.cursor_row -= rows;
        } else {
            self.cursor_row = 0;
        }
        if self.scroll_row >= rows {
            self.scroll_row -= rows;
        } else {
            self.scroll_row = 0;
        }
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
    }

    fn page_down(&mut self) {
        let rows = EDITOR_ROWS;
        let last = self.lines.len().saturating_sub(1);
        self.cursor_row = (self.cursor_row + rows).min(last);
        self.scroll_row = (self.scroll_row + rows).min(last.saturating_sub(rows - 1));
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col > line_len {
            self.cursor_col = line_len;
        }
    }

    // -----------------------------------------------------------------------
    // Editing operations
    // -----------------------------------------------------------------------

    /// Insert a printable character at the current cursor position.
    fn insert_char(&mut self, c: u8) {
        // Prevent unreasonable line growth
        let line_len = self.lines[self.cursor_row].len();
        if line_len >= EDITOR_COLS * 4 { return; }
        // Prevent too many lines
        if self.lines.len() >= MAX_LINES { return; }
        self.lines[self.cursor_row].insert(self.cursor_col, c);
        self.cursor_col += 1;
        self.modified = true;
    }

    /// Insert a newline: split current line at cursor.
    fn insert_newline(&mut self) {
        if self.lines.len() >= MAX_LINES { return; }
        let rest: Vec<u8> = self.lines[self.cursor_row].split_off(self.cursor_col);
        self.cursor_row += 1;
        self.lines.insert(self.cursor_row, rest);
        self.cursor_col = 0;
        self.modified = true;
    }

    /// Delete character before cursor (Backspace).
    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.lines[self.cursor_row].remove(self.cursor_col - 1);
            self.cursor_col -= 1;
            self.modified = true;
        } else if self.cursor_row > 0 {
            // Merge with previous line
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].extend_from_slice(&current_line);
            self.modified = true;
        }
    }

    /// Delete character at cursor (Delete key).
    fn delete_char(&mut self) {
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col < line_len {
            self.lines[self.cursor_row].remove(self.cursor_col);
            self.modified = true;
        } else if self.cursor_row + 1 < self.lines.len() {
            // Merge next line into current
            let next_line = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].extend_from_slice(&next_line);
            self.modified = true;
        }
    }

    /// Kill line from cursor to end (Ctrl+K).
    fn kill_to_end(&mut self) {
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col < line_len {
            self.lines[self.cursor_row].truncate(self.cursor_col);
            self.modified = true;
        } else if self.cursor_row + 1 < self.lines.len() {
            // At end of line: merge next line
            let next_line = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].extend_from_slice(&next_line);
            self.modified = true;
        }
    }

    // -----------------------------------------------------------------------
    // File I/O
    // -----------------------------------------------------------------------

    /// Serialize buffer to bytes and write to VFS.
    fn save(&mut self) {
        let mut data: Vec<u8> = Vec::new();
        for (i, line) in self.lines.iter().enumerate() {
            data.extend_from_slice(line);
            if i + 1 < self.lines.len() {
                data.push(b'\n');
            }
        }
        match crate::vfs::write_file(&self.filename, &data) {
            Ok(()) => {
                self.modified = false;
                self.show_message("File saved.", Color::LightGreen);
            }
            Err(_) => {
                self.show_message("ERROR: Save failed!", Color::LightRed);
            }
        }
    }

    /// Briefly show a message on the status bar (renders once and then re-renders normally).
    fn show_message(&self, msg: &str, color: Color) {
        let bar_color = vga::make_color(color, Color::Black);
        let mut col = 0;
        for b in msg.bytes() {
            if col >= EDITOR_COLS { break; }
            unsafe { vga_putchar_at(b, bar_color, col, STATUS_ROW - 1); }
            col += 1;
        }
        while col < EDITOR_COLS {
            unsafe { vga_putchar_at(b' ', bar_color, col, STATUS_ROW - 1); }
            col += 1;
        }
        self.update_hw_cursor();
        // Small delay for visual feedback
        for _ in 0..5_000_000u32 {
            core::hint::spin_loop();
        }
    }

    /// Ask the user "Unsaved changes. Quit? (y/n)" on the status bar.
    fn confirm_quit(&self) -> bool {
        let bar_color = vga::make_color(Color::Yellow, Color::Black);
        let msg = b"Unsaved changes! Quit anyway? (y/n): ";
        let mut col = 0;
        for &b in msg {
            if col >= EDITOR_COLS { break; }
            unsafe { vga_putchar_at(b, bar_color, col, STATUS_ROW - 1); }
            col += 1;
        }
        while col < EDITOR_COLS {
            unsafe { vga_putchar_at(b' ', bar_color, col, STATUS_ROW - 1); }
            col += 1;
        }
        self.update_hw_cursor();

        loop {
            let key = unsafe { keyboard_getchar() };
            match key {
                k if k == b'y' as u16 || k == b'Y' as u16 => return true,
                k if k == b'n' as u16 || k == b'N' as u16 => return false,
                KEY_CTRL_Q => return true,
                _ => {}
            }
        }
    }

    // -----------------------------------------------------------------------
    // Main event loop
    // -----------------------------------------------------------------------

    /// Run the editor event loop. Returns when user quits.
    fn run(&mut self) {
        vga::clear_screen();
        self.render_all();

        loop {
            let key = unsafe { keyboard_getchar() };

            match key {
                // Navigation
                KEY_UP         => { self.move_up();    }
                KEY_DOWN       => { self.move_down();  }
                KEY_LEFT       => { self.move_left();  }
                KEY_RIGHT      => { self.move_right(); }
                KEY_HOME | KEY_CTRL_A => { self.move_home(); }
                KEY_END  | KEY_CTRL_E => { self.move_end(); }
                KEY_PAGE_UP    => { self.page_up();    }
                KEY_PAGE_DOWN  => { self.page_down();  }

                // Editing
                KEY_ENTER | KEY_RETURN => { self.insert_newline(); }
                KEY_BACKSPACE | KEY_DEL_CHAR => { self.backspace(); }
                KEY_DELETE => { self.delete_char(); }
                KEY_CTRL_K => { self.kill_to_end(); }

                // Save
                KEY_CTRL_S => { self.save(); }

                // Quit
                KEY_CTRL_Q => {
                    if !self.modified || self.confirm_quit() {
                        break;
                    }
                }

                // Printable ASCII
                c if c >= 32 && c <= 126 => {
                    self.insert_char(c as u8);
                }

                _ => { /* ignore unhandled keys */ }
            }

            self.scroll_to_cursor();
            self.render_all();
        }

        // Restore shell screen
        vga::clear_screen();
    }
}

/// Helper: write an unsigned integer to VGA at (col, row), return new col.
fn write_number_at(mut n: usize, mut col: usize, row: usize, color: u8) -> usize {
    if n == 0 {
        unsafe { vga_putchar_at(b'0', color, col, row); }
        return col + 1;
    }
    let mut buf = [0u8; 8];
    let mut len = 0;
    while n > 0 && len < 8 {
        buf[len] = b'0' + (n % 10) as u8;
        n /= 10;
        len += 1;
    }
    for i in (0..len).rev() {
        if col < EDITOR_COLS {
            unsafe { vga_putchar_at(buf[i], color, col, row); }
            col += 1;
        }
    }
    col
}

/// Public entry point called from commands.rs.
pub fn open(filename: &str) {
    // Load existing content from VFS if the file exists
    let content: Vec<u8> = match crate::vfs::read_file(filename) {
        Some(data) => data,
        None => Vec::new(),
    };

    let mut editor = Editor::new(filename, &content);
    editor.run();
}
