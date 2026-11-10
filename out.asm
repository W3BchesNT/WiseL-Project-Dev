include 'kernel/FASM/fasmg/extension/format.inc'
include 'kernel/FASM/fasmg/extension/wir_ll.inc'

set_build X64, Windows, Console

entry start

include "win64w.inc"

data_block
    WVAR hStdOut, i64, ?
    WVAR bytesWritten, i32, ?

    WSTR arg1_str_fn_print_str, 'Hello, World!', 13, 10
    WSTR_LEN arg1_str_fn_print_str

code_block

start:
    push rbp
    mov rbp, rsp
    ; WindowsShadowSpace()
    sub rsp, 8
    and rsp, -16
    sub rsp, 40
    
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
    ; Exit()
    invoke ExitProcess, 0
.epilogue_0:
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
