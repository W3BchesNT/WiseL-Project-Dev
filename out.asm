include 'kernel/FASM/fasmg/extension/format.inc'
include 'kernel/FASM/fasmg/extension/wir_ll.inc'

set_build X64, Linux, Console

entry start

data_block
    WVAR a, i32, 10
    WVAR b, i32, 20
    WSTR c, 'Hello, World!', 0
    WSTR_LEN c
    WSTR hello_msg, 'Hello, WiseL!', 0
    WSTR_LEN hello_msg

code_block
start:
    push rbp
    mov rbp, rsp
    ; syscall(1, 1, hello_msg, hello_msg_len)
    mov rax, 1
    mov rdi, 1
    lea rsi, [hello_msg]
    mov rdx, hello_msg_len
    syscall
    ; Exit()
    mov rax, 60
    xor rdi, rdi
    syscall
    
.epilogue_0:
    mov rsp, rbp
    pop rbp
    ret

