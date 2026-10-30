include 'kernel/FASM/fasmg/extension/format.inc'
include 'kernel/FASM/fasmg/extension/wir_ll.inc'

set_build X64, Windows, Console

entry start

include "win64w.inc"

data_block
    WVAR hStdOut, i64, ?
    WVAR bytesWritten, i32, ?

    WSTR str_hi, 'Hello, World!', 13, 10
    WSTR_LEN str_hi
    WSTR str_hru, 'How are you?', 13, 10
    WSTR_LEN str_hru
    WSTR str_new, 'What are you doing?', 13, 10
    WSTR_LEN str_new
    WSTR str_by, 'Bye, World!', 13, 10
    WSTR_LEN str_by
    a dd 10

code_block

start:
    mov eax, [a]
    mov rcx, 10
    cmp eax, ecx
    sete al
    movzx eax, al
    cmp rax, 0
    je .else_0
    sub rsp, 40
    mov rcx, -11
    call qword [GetStdHandle]
    mov rcx, rax
    lea rdx, [str_hi]
    mov r8, 15
    lea r9, [rsp+32]
    mov qword [rsp+32], 0
    call qword [WriteConsoleA]
    add rsp, 40
    jmp .end_0
.else_0:
    sub rsp, 40
    mov rcx, -11
    call qword [GetStdHandle]
    mov rcx, rax
    lea rdx, [str_by]
    mov r8, 13
    lea r9, [rsp+32]
    mov qword [rsp+32], 0
    call qword [WriteConsoleA]
    add rsp, 40
.end_0:
    sub rsp, 40
    mov rcx, -11
    call qword [GetStdHandle]
    mov rcx, rax
    lea rdx, [str_hru]
    mov r8, 14
    lea r9, [rsp+32]
    mov qword [rsp+32], 0
    call qword [WriteConsoleA]
    add rsp, 40
    sub rsp, 40
    mov rcx, -11
    call qword [GetStdHandle]
    mov rcx, rax
    lea rdx, [str_new]
    mov r8, 21
    lea r9, [rsp+32]
    mov qword [rsp+32], 0
    call qword [WriteConsoleA]
    add rsp, 40
    invoke ExitProcess, 0

import_block
    library kernel32, 'KERNEL32.DLL'
    import kernel32, \
        GetStdHandle, 'GetStdHandle', \
        WriteConsoleA, 'WriteConsoleA', \
        ExitProcess, 'ExitProcess'
