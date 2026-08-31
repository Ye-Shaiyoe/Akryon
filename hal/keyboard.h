#ifndef AKRYON_HAL_KEYBOARD_H
#define AKRYON_HAL_KEYBOARD_H

#include "types.h"

// Special extended keycodes (0x0100+)
#define KEY_UP        0x0100
#define KEY_DOWN      0x0101
#define KEY_LEFT      0x0102
#define KEY_RIGHT     0x0103
#define KEY_HOME      0x0104
#define KEY_END       0x0105
#define KEY_PAGE_UP   0x0106
#define KEY_PAGE_DOWN 0x0107
#define KEY_INSERT    0x0108
#define KEY_DELETE    0x0109

// Standard Control Key character constants
#define KEY_CTRL_A    0x0001
#define KEY_CTRL_B    0x0002
#define KEY_CTRL_C    0x0003
#define KEY_CTRL_D    0x0004
#define KEY_CTRL_E    0x0005
#define KEY_CTRL_F    0x0006
#define KEY_CTRL_G    0x0007
#define KEY_CTRL_H    0x0008 // Backspace (\b)
#define KEY_CTRL_I    0x0009 // Tab (\t)
#define KEY_CTRL_J    0x000A // Newline (\n)
#define KEY_CTRL_K    0x000B
#define KEY_CTRL_L    0x000C // Form Feed (Ctrl+L)
#define KEY_CTRL_M    0x000D // Carriage Return (\r)
#define KEY_CTRL_U    0x0015
#define KEY_CTRL_W    0x0017

void keyboard_init(void);
uint16_t keyboard_getchar(void);
bool keyboard_has_char(void);

#endif // AKRYON_HAL_KEYBOARD_H

