; ==============================================================================
; Akryon OS - Universal MBR Bootloader (LBA 64-sector chunk loader)
; 16-bit Real Mode -> 32-bit Protected Mode Loader
; ==============================================================================

[BITS 16]
[ORG 0x7C00]

KERNEL_START_SEG equ 0x1000      ; 0x1000:0x0000 -> Physical 0x10000 (64 KB)
TOTAL_SECTORS    equ 800         ; Total sektor kernel (400 KB)
CHUNK_SECTORS    equ 64          ; Baca dalam chunk 64 sektor (32 KB) per int 0x13

start:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    sti

    mov [boot_drive], dl

    ; Cetak pesan boot
    mov si, msg_boot
    call print_string_16

    ; Muat kernel menggunakan LBA chunk loader
    call load_kernel_chunks

    ; Aktifkan A20 Gate
    call enable_a20

    ; Matikan interrupt sebelum lompat ke Protected Mode
    cli

    ; Load GDT
    lgdt [gdt_descriptor]

    ; Enable Protected Mode
    mov eax, cr0
    or eax, 1
    mov cr0, eax

    ; Far jump ke 32-bit code segment
    jmp CODE_SEG:protected_mode_entry

; ------------------------------------------------------------------------------
; Print String 16-bit
; ------------------------------------------------------------------------------
print_string_16:
    pusha
.loop:
    lodsb
    test al, al
    jz .done
    mov ah, 0x0E
    mov bh, 0x00
    mov bl, 0x07
    int 0x10
    jmp .loop
.done:
    popa
    ret

; ------------------------------------------------------------------------------
; LBA Chunk Loader: Membaca 64 sektor per int 0x13, ah=0x42
; ------------------------------------------------------------------------------
load_kernel_chunks:
    pusha

    mov word [dap_segment], KERNEL_START_SEG
    mov dword [dap_lba_low], 1      ; Mulai dari LBA 1
    mov dword [dap_lba_high], 0
    mov word [sectors_remaining], TOTAL_SECTORS

.chunk_loop:
    cmp word [sectors_remaining], 0
    jle .load_done

    ; Tentukan jumlah sektor untuk chunk ini: min(sectors_remaining, CHUNK_SECTORS)
    mov ax, CHUNK_SECTORS
    cmp ax, [sectors_remaining]
    jle .count_ok
    mov ax, [sectors_remaining]
.count_ok:
    mov [dap_num_sectors], ax

    ; Panggil BIOS Extended Read (int 0x13, AH=0x42)
    mov ah, 0x42
    mov dl, [boot_drive]
    mov si, disk_address_packet
    int 0x13
    jc .disk_error

    ; Kurangi sektor tersisa
    mov ax, [dap_num_sectors]
    sub [sectors_remaining], ax

    ; Majukan LBA starting sector
    add [dap_lba_low], eax

    ; Majukan segment tujuan: (num_sectors * 512) / 16 = num_sectors * 32
    shl ax, 5                       ; AX = num_sectors * 32
    add [dap_segment], ax

    jmp .chunk_loop

.disk_error:
    mov si, msg_disk_err
    call print_string_16
    hlt
    jmp $

.load_done:
    popa
    ret

; ------------------------------------------------------------------------------
; Enable A20 Gate
; ------------------------------------------------------------------------------
enable_a20:
    in al, 0x92
    test al, 2
    jnz .done
    or al, 2
    and al, 0xFE
    out 0x92, al
.done:
    ret

; ------------------------------------------------------------------------------
; GDT Table
; ------------------------------------------------------------------------------
gdt_start:
    dd 0x0, 0x0

gdt_code:
    dw 0xFFFF
    dw 0x0000
    db 0x00
    db 10011010b
    db 11001111b
    db 0x00

gdt_data:
    dw 0xFFFF
    dw 0x0000
    db 0x00
    db 10010010b
    db 11001111b
    db 0x00

gdt_end:

gdt_descriptor:
    dw gdt_end - gdt_start - 1
    dd gdt_start

CODE_SEG equ gdt_code - gdt_start
DATA_SEG equ gdt_data - gdt_start

; ------------------------------------------------------------------------------
; 32-bit Protected Mode Entry
; ------------------------------------------------------------------------------
[BITS 32]
protected_mode_entry:
    mov ax, DATA_SEG
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    mov esp, 0x90000

    jmp 0x10000

; ------------------------------------------------------------------------------
; Disk Address Packet (DAP)
; ------------------------------------------------------------------------------
align 4
disk_address_packet:
    db 0x10                         ; Size of packet (16 bytes)
    db 0x00                         ; Reserved (0)
dap_num_sectors:
    dw CHUNK_SECTORS                ; Sektor yang dibaca per panggilan (64)
dap_offset:
    dw 0x0000                       ; Offset buffer (selalu 0x0000)
dap_segment:
    dw KERNEL_START_SEG             ; Segment buffer (dimajukan setiap chunk)
dap_lba_low:
    dd 1                            ; LBA Low
dap_lba_high:
    dd 0                            ; LBA High

; ------------------------------------------------------------------------------
; Data Variables
; ------------------------------------------------------------------------------
boot_drive:        db 0
sectors_remaining: dw 0

msg_boot:          db "[Akryon] Booting Akryon OS...", 13, 10, 0
msg_disk_err:      db "[ERROR] Disk read error!", 13, 10, 0

times 510-($-$$) db 0
dw 0xAA55
