#include "vga.h"
#include "io.h"

static volatile uint16_t* const VGA_BUFFER = (volatile uint16_t*)VGA_BUFFER_ADDR;
static size_t cursor_col = 0;
static size_t cursor_row = 0;
static uint8_t current_color = 0x07; // Light gray on black

void vga_init(void) {
    current_color = vga_make_color(VGA_COLOR_WHITE, VGA_COLOR_BLACK);
    vga_clear();
}

void vga_clear(void) {
    for (size_t y = 0; y < VGA_HEIGHT; y++) {
        for (size_t x = 0; x < VGA_WIDTH; x++) {
            const size_t index = y * VGA_WIDTH + x;
            VGA_BUFFER[index] = vga_make_entry(' ', current_color);
        }
    }
    cursor_col = 0;
    cursor_row = 0;
    vga_update_cursor();
}

void vga_set_color(uint8_t fg, uint8_t bg) {
    current_color = vga_make_color((vga_color_t)fg, (vga_color_t)bg);
}

void vga_set_cursor(size_t x, size_t y) {
    if (x < VGA_WIDTH) cursor_col = x;
    if (y < VGA_HEIGHT) cursor_row = y;
    vga_update_cursor();
}

void vga_get_cursor(size_t* x, size_t* y) {
    if (x) *x = cursor_col;
    if (y) *y = cursor_row;
}

void vga_update_cursor(void) {
    uint16_t pos = (uint16_t)(cursor_row * VGA_WIDTH + cursor_col);
    outb(0x3D4, 0x0F);
    outb(0x3D5, (uint8_t)(pos & 0xFF));
    outb(0x3D4, 0x0E);
    outb(0x3D5, (uint8_t)((pos >> 8) & 0xFF));
}

static void vga_scroll(void) {
    for (size_t y = 0; y < VGA_HEIGHT - 1; y++) {
        for (size_t x = 0; x < VGA_WIDTH; x++) {
            VGA_BUFFER[y * VGA_WIDTH + x] = VGA_BUFFER[(y + 1) * VGA_WIDTH + x];
        }
    }
    for (size_t x = 0; x < VGA_WIDTH; x++) {
        VGA_BUFFER[(VGA_HEIGHT - 1) * VGA_WIDTH + x] = vga_make_entry(' ', current_color);
    }
    cursor_row = VGA_HEIGHT - 1;
}

void vga_putchar_at(char c, uint8_t color, size_t x, size_t y) {
    if (x < VGA_WIDTH && y < VGA_HEIGHT) {
        VGA_BUFFER[y * VGA_WIDTH + x] = vga_make_entry((unsigned char)c, color);
    }
}

void vga_putchar(char c) {
    if (c == '\n') {
        cursor_col = 0;
        cursor_row++;
    } else if (c == '\r') {
        cursor_col = 0;
    } else if (c == '\t') {
        cursor_col = (cursor_col + 4) & ~(4 - 1);
        if (cursor_col >= VGA_WIDTH) {
            cursor_col = 0;
            cursor_row++;
        }
    } else if (c == '\b') {
        vga_backspace();
        return;
    } else {
        VGA_BUFFER[cursor_row * VGA_WIDTH + cursor_col] = vga_make_entry((unsigned char)c, current_color);
        cursor_col++;
        if (cursor_col >= VGA_WIDTH) {
            cursor_col = 0;
            cursor_row++;
        }
    }

    if (cursor_row >= VGA_HEIGHT) {
        vga_scroll();
    }

    vga_update_cursor();
}

void vga_backspace(void) {
    if (cursor_col > 0) {
        cursor_col--;
        VGA_BUFFER[cursor_row * VGA_WIDTH + cursor_col] = vga_make_entry(' ', current_color);
        vga_update_cursor();
    } else if (cursor_row > 0) {
        cursor_row--;
        cursor_col = VGA_WIDTH - 1;
        VGA_BUFFER[cursor_row * VGA_WIDTH + cursor_col] = vga_make_entry(' ', current_color);
        vga_update_cursor();
    }
}

void vga_puts(const char* str) {
    if (!str) return;
    for (size_t i = 0; str[i] != '\0'; i++) {
        vga_putchar(str[i]);
    }
}

void vga_puts_colored(const char* str, uint8_t fg, uint8_t bg) {
    uint8_t old_color = current_color;
    vga_set_color(fg, bg);
    vga_puts(str);
    current_color = old_color;
}

void vga_puthex(uint32_t val) {
    const char hex_digits[] = "0123456789ABCDEF";
    vga_puts("0x");
    for (int i = 28; i >= 0; i -= 4) {
        vga_putchar(hex_digits[(val >> i) & 0xF]);
    }
}

void vga_putdec(uint32_t val) {
    if (val == 0) {
        vga_putchar('0');
        return;
    }
    char buf[12];
    int i = 0;
    while (val > 0) {
        buf[i++] = (char)('0' + (val % 10));
        val /= 10;
    }
    for (int j = i - 1; j >= 0; j--) {
        vga_putchar(buf[j]);
    }
}
