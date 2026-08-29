#include "keyboard.h"
#include "isr.h"
#include "io.h"

#define KEYBOARD_BUFFER_SIZE 256

static volatile char key_buffer[KEYBOARD_BUFFER_SIZE];
static volatile uint32_t buf_head = 0;
static volatile uint32_t buf_tail = 0;

static bool shift_pressed = false;
static bool caps_lock = false;

// US QWERTY Scancode Set 1 standard map
static const char kbd_us_lower[128] = {
    0,   27,  '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '-', '=', '\b',
    '\t', 'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', '[', ']', '\n',
    0,   'a', 's', 'd', 'f', 'g', 'h', 'j', 'k', 'l', ';', '\'', '`',
    0,   '\\', 'z', 'x', 'c', 'v', 'b', 'n', 'm', ',', '.', '/', 0,
    '*', 0,   ' ', 0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
    0,   0,   '7', '8', '9', '-', '4', '5', '6', '+', '1', '2', '3', '0',
    '.', 0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
    0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
    0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0
};

static const char kbd_us_upper[128] = {
    0,   27,  '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '_', '+', '\b',
    '\t', 'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P', '{', '}', '\n',
    0,   'A', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L', ':', '"', '~',
    0,   '|', 'Z', 'X', 'C', 'V', 'B', 'N', 'M', '<', '>', '?', 0,
    '*', 0,   ' ', 0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
    0,   0,   '7', '8', '9', '-', '4', '5', '6', '+', '1', '2', '3', '0',
    '.', 0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
    0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,
    0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0,   0
};

static void keyboard_callback(registers_t* regs) {
    (void)regs;
    uint8_t scancode = inb(0x60);

    // Left Shift (0x2A) or Right Shift (0x36) pressed
    if (scancode == 0x2A || scancode == 0x36) {
        shift_pressed = true;
        return;
    }
    // Left Shift (0xAA) or Right Shift (0xB6) released
    if (scancode == 0xAA || scancode == 0xB6) {
        shift_pressed = false;
        return;
    }
    // CapsLock (0x3A) pressed
    if (scancode == 0x3A) {
        caps_lock = !caps_lock;
        return;
    }

    // Key released (high bit set) -> ignore
    if (scancode & 0x80) {
        return;
    }

    if (scancode < 128) {
        char ch = 0;
        bool upper = (shift_pressed ^ caps_lock);

        if (upper) {
            ch = kbd_us_upper[scancode];
        } else {
            ch = kbd_us_lower[scancode];
        }

        if (ch != 0) {
            uint32_t next_head = (buf_head + 1) % KEYBOARD_BUFFER_SIZE;
            if (next_head != buf_tail) {
                key_buffer[buf_head] = ch;
                buf_head = next_head;
            }
        }
    }
}

void keyboard_init(void) {
    buf_head = 0;
    buf_tail = 0;
    shift_pressed = false;
    caps_lock = false;
    isr_register_handler(33, keyboard_callback); // IRQ 1 = Vector 33
}

bool keyboard_has_char(void) {
    return (buf_head != buf_tail);
}

char keyboard_getchar(void) {
    while (!keyboard_has_char()) {
        hlt();
    }
    char ch = key_buffer[buf_tail];
    buf_tail = (buf_tail + 1) % KEYBOARD_BUFFER_SIZE;
    return ch;
}
