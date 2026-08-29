#ifndef AKRYON_HAL_KEYBOARD_H
#define AKRYON_HAL_KEYBOARD_H

#include "types.h"

void keyboard_init(void);
char keyboard_getchar(void);
bool keyboard_has_char(void);

#endif // AKRYON_HAL_KEYBOARD_H
