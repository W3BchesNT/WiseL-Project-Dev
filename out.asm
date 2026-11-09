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
    WVAR a, i32, 10
    WSTR arg1_str_fn_println, 'Hello, World!', 13, 10
    WSTR_LEN arg1_str_fn_println
    WSTR arg2_str_fn_println, 'Bye, World!', 13, 10
    WSTR_LEN arg2_str_fn_println

code_block

start:
    ; WindowsShadowSpace()
    pop rax
    sub rsp, 8
    and rsp, -16
    sub rsp, 40
    push rax
    
    ; println(str_new)
    sub rsp, 40
    mov rcx, -11
    call qword [GetStdHandle]
    mov rcx, rax
    lea rdx, [str_new]
    mov r8, str_new_len
    lea r9, [rsp+32]
    mov qword [rsp+32], 0
    call qword [WriteConsoleA]
    add rsp, 40
    ; if (a > 1)
    mov eax, [a]
    mov rcx, 1
    cmp rax, rcx
    setg al
    movzx rax, al
    cmp rax, 0
    je .else_0
    ; println("Hello, World!")
    sub rsp, 40
    mov rcx, -11
    call qword [GetStdHandle]
    mov rcx, rax
    lea rdx, [arg1_str_fn_println]
    mov r8, arg1_str_fn_println_len
    lea r9, [rsp+32]
    mov qword [rsp+32], 0
    call qword [WriteConsoleA]
    add rsp, 40
    jmp .end_1
.else_0:
    ; println("Bye, World!")
    sub rsp, 40
    mov rcx, -11
    call qword [GetStdHandle]
    mov rcx, rax
    lea rdx, [arg2_str_fn_println]
    mov r8, arg2_str_fn_println_len
    lea r9, [rsp+32]
    mov qword [rsp+32], 0
    call qword [WriteConsoleA]
    add rsp, 40
.end_1:
    ; endline()
    sub rsp, 312
    mov rcx, -10
    call qword [GetStdHandle]
    mov rcx, rax
    lea rdx, [rsp+40]
    mov r8, 255
    lea r9, [rsp+296]
    mov qword [r9], 0
    call qword [ReadConsoleA]
    add rsp, 312
    ; Exit()
    invoke ExitProcess, 0

import_block
    library kernel32, 'KERNEL32.DLL'
    import kernel32, \
        GetStdHandle, 'GetStdHandle', \
        WriteConsoleA, 'WriteConsoleA', \
        ExitProcess, 'ExitProcess', \
        ReadConsoleA, 'ReadConsoleA'
