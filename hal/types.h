#ifndef AKRYON_HAL_TYPES_H
#define AKRYON_HAL_TYPES_H

typedef unsigned char      uint8_t;
typedef unsigned short     uint16_t;
typedef unsigned int       uint32_t;
typedef unsigned long long uint64_t;

typedef signed char        int8_t;
typedef signed short       int16_t;
typedef signed int         int32_t;
typedef signed long long   int64_t;

typedef uint32_t           size_t;
typedef int32_t            ssize_t;
typedef uint32_t           uintptr_t;
typedef int32_t            intptr_t;

#define NULL ((void*)0)

#if !defined(__cplusplus) && (!defined(__STDC_VERSION__) || __STDC_VERSION__ < 202311L)
typedef _Bool bool;
#define true 1
#define false 0
#endif

void*  memset(void* dest, int val, size_t count);
void*  memcpy(void* dest, const void* src, size_t count);
void*  memmove(void* dest, const void* src, size_t count);
int    memcmp(const void* s1, const void* s2, size_t count);
int    bcmp(const void* s1, const void* s2, size_t count);
size_t strlen(const char* str);
int    strcmp(const char* s1, const char* s2);
int    strncmp(const char* s1, const char* s2, size_t n);

#endif // AKRYON_HAL_TYPES_H
