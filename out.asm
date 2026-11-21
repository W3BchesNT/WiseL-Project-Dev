include 'kernel/FASM/fasmg/extension/format.inc'
include 'kernel/FASM/fasmg/extension/wir_ll.inc'

set_build X64, Windows, Console

entry start

include "win64w.inc"

data_block
    WVAR hStdOut, i64, ?
    readln_buf rb 256
    readln_count dd 0
    newline_buf db 13, 10

    WVAR a, i32, 10
    WVAR b, i32, 20
    WSTR c, 'Hello, World!', 0
    WSTR_LEN c
    WSTR hello_msg, 'Hello, WiseL!', 0
    WSTR_LEN hello_msg
    WVAR bytesWritten, i32, 0

code_block
start:
    push rbp
    mov rbp, rsp
    ; WindowsShadowSpace()
    and rsp, -16
    sub rsp, 48
    ; let hStdOut: any = GetStdHandle(11)
    invoke GetStdHandle, -11
    mov rax, rax
    mov [rbp - 8], rax
    mov [hStdOut], rax
    ; WriteConsoleA(hStdOut, hello_msg, hello_msg_len, bytesWritten)
    invoke WriteConsoleA, [hStdOut], hello_msg, hello_msg_len, bytesWritten, 0
    ; ExitProcess()
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
