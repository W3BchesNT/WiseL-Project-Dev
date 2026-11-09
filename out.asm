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
    WSTR arg1_str_fn_print_str, 'Hello, World!', 13, 10
    WSTR_LEN arg1_str_fn_print_str
    WSTR arg2_str_fn_print_str, 'Bye, World!', 13, 10
    WSTR_LEN arg2_str_fn_print_str

code_block

fn_add:
    push rbp
    mov rbp, rsp
    sub rsp, 16
    mov [rbp - 8], ecx
    mov [rbp - 16], edx
    ; return a + b
    mov rax, [rbp - 8]
    mov rcx, [rbp - 16]
    add rax, rcx
    mov rax, rax
    jmp .epilogue_0
.epilogue_0:
    mov rsp, rbp
    pop rbp
    ret

start:
    push rbp
    mov rbp, rsp
    sub rsp, 16
    ; WindowsShadowSpace()
    sub rsp, 8
    and rsp, -16
    sub rsp, 40
    
    ; let mo: i32 = add(2, 3)
    mov rcx, 2
    mov rdx, 3
    call fn_add
    mov eax, eax
    mov [rbp - 8], rax
    ; print_int(mo)
    sub rsp, 40
    mov rcx, -11
    call qword [GetStdHandle]
    mov rcx, rax
    lea rdx, [rsp + 32]
    mov eax, [rbp - 8]
    add al, '0'
    mov [rdx], al
    mov byte [rdx + 1], 10
    mov r8, 2
    lea r9, [rsp + 24]
    mov qword [r9], 0
    call qword [WriteConsoleA]
    add rsp, 40
    ; print_str(str_new)
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
    je .else_2
    ; print_str("Hello, World!")
    sub rsp, 40
    mov rcx, -11
    call qword [GetStdHandle]
    mov rcx, rax
    lea rdx, [arg1_str_fn_print_str]
    mov r8, arg1_str_fn_print_str_len
    lea r9, [rsp+32]
    mov qword [rsp+32], 0
    call qword [WriteConsoleA]
    add rsp, 40
    jmp .end_3
.else_2:
    ; print_str("Bye, World!")
    sub rsp, 40
    mov rcx, -11
    call qword [GetStdHandle]
    mov rcx, rax
    lea rdx, [arg2_str_fn_print_str]
    mov r8, arg2_str_fn_print_str_len
    lea r9, [rsp+32]
    mov qword [rsp+32], 0
    call qword [WriteConsoleA]
    add rsp, 40
.end_3:
    ; Exit()
    invoke ExitProcess, 0
.epilogue_1:
    mov rsp, rbp
    pop rbp
    ret

import_block
    library kernel32, 'KERNEL32.DLL'
    import kernel32, \
        GetStdHandle, 'GetStdHandle', \
        WriteConsoleA, 'WriteConsoleA', \
        ExitProcess, 'ExitProcess', \
        ReadConsoleA, 'ReadConsoleA'
