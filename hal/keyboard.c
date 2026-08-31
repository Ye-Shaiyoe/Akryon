#include "keyboard.h"
#include "isr.h"
#include "io.h"

#define KEYBOARD_BUFFER_SIZE 256

static volatile uint16_t key_buffer[KEYBOARD_BUFFER_SIZE];
static volatile uint32_t buf_head = 0;
static volatile uint32_t buf_tail = 0;

static bool shift_pressed = false;
static bool ctrl_pressed = false;
static bool alt_pressed = false;
static bool caps_lock = false;
static bool extended_e0 = false;

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

static void keyboard_push(uint16_t key) {
    uint32_t next_head = (buf_head + 1) % KEYBOARD_BUFFER_SIZE;
    if (next_head != buf_tail) {
        key_buffer[buf_head] = key;
        buf_head = next_head;
    }
}

static void keyboard_callback(registers_t* regs) {
    (void)regs;
    uint8_t scancode = inb(0x60);

    // Check for extended scancode prefix (0xE0)
    if (scancode == 0xE0) {
        extended_e0 = true;
        return;
    }

    if (extended_e0) {
        extended_e0 = false;

        // Extended key release (high bit set)
        if (scancode & 0x80) {
            uint8_t released = scancode & 0x7F;
            if (released == 0x1D) {
                ctrl_pressed = false;
            } else if (released == 0x38) {
                alt_pressed = false;
            }
            return;
        }

        // Extended key press
        if (scancode == 0x1D) {
            ctrl_pressed = true;
            return;
        }
        if (scancode == 0x38) {
            alt_pressed = true;
            return;
        }

        uint16_t special_key = 0;
        switch (scancode) {
            case 0x48: special_key = KEY_UP; break;
            case 0x50: special_key = KEY_DOWN; break;
            case 0x4B: special_key = KEY_LEFT; break;
            case 0x4D: special_key = KEY_RIGHT; break;
            case 0x47: special_key = KEY_HOME; break;
            case 0x4F: special_key = KEY_END; break;
            case 0x49: special_key = KEY_PAGE_UP; break;
            case 0x51: special_key = KEY_PAGE_DOWN; break;
            case 0x52: special_key = KEY_INSERT; break;
            case 0x53: special_key = KEY_DELETE; break;
            case 0x1C: special_key = '\n'; break; // Keypad Enter
            case 0x35: special_key = '/'; break;  // Keypad /
            default: break;
        }

        if (special_key != 0) {
            keyboard_push(special_key);
        }
        return;
    }

    // Normal (non-extended) scancodes
    // Left Ctrl (0x1D pressed, 0x9D released)
    if (scancode == 0x1D) {
        ctrl_pressed = true;
        return;
    }
    if (scancode == 0x9D) {
        ctrl_pressed = false;
        return;
    }

    // Left Shift (0x2A), Right Shift (0x36)
    if (scancode == 0x2A || scancode == 0x36) {
        shift_pressed = true;
        return;
    }
    if (scancode == 0xAA || scancode == 0xB6) {
        shift_pressed = false;
        return;
    }

    // Left Alt (0x38 pressed, 0xB8 released)
    if (scancode == 0x38) {
        alt_pressed = true;
        return;
    }
    if (scancode == 0xB8) {
        alt_pressed = false;
        return;
    }

    // CapsLock (0x3A)
    if (scancode == 0x3A) {
        caps_lock = !caps_lock;
        return;
    }

    // Key released (high bit set)
    if (scancode & 0x80) {
        return;
    }

    if (scancode < 128) {
        // If Control is active
        if (ctrl_pressed) {
            char lower_char = kbd_us_lower[scancode];
            if (lower_char >= 'a' && lower_char <= 'z') {
                // Ctrl+A -> 1, Ctrl+C -> 3, Ctrl+L -> 12, etc.
                uint16_t ctrl_code = (uint16_t)(lower_char - 'a' + 1);
                keyboard_push(ctrl_code);
                return;
            }
            if (lower_char == '[') { keyboard_push(27); return; } // ESC
            if (lower_char == '\\') { keyboard_push(28); return; }
            if (lower_char == ']') { keyboard_push(29); return; }
        }

        // Standard character
        char base = kbd_us_lower[scancode];
        char ch = 0;
        if (base >= 'a' && base <= 'z') {
            bool upper = (shift_pressed ^ caps_lock);
            ch = upper ? kbd_us_upper[scancode] : kbd_us_lower[scancode];
        } else {
            ch = shift_pressed ? kbd_us_upper[scancode] : kbd_us_lower[scancode];
        }

        if (ch != 0) {
            keyboard_push((uint16_t)(unsigned char)ch);
        }
    }
}

void keyboard_init(void) {
    buf_head = 0;
    buf_tail = 0;
    shift_pressed = false;
    ctrl_pressed = false;
    alt_pressed = false;
    caps_lock = false;
    extended_e0 = false;
    isr_register_handler(33, keyboard_callback); // IRQ 1 = Vector 33
}

bool keyboard_has_char(void) {
    return (buf_head != buf_tail);
}

uint16_t keyboard_getchar(void) {
    while (!keyboard_has_char()) {
        hlt();
    }
    uint16_t ch = key_buffer[buf_tail];
    buf_tail = (buf_tail + 1) % KEYBOARD_BUFFER_SIZE;
    return ch;
}

