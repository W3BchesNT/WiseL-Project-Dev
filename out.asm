include 'kernel/FASM/fasmg/extension/format.inc'
include 'kernel/FASM/fasmg/extension/wir_ll.inc'

set_build X64, Linux, Console
entry start

data_block
    LINUX_WSTR str_hello , 'Hello, World My!' , 13 , 10

code_block

fn_Exit:
    mov rax , 60
    xor rdi , rdi
    syscall
    ret

start:
    mov rax , 1
    mov rdi , 1
    lea rsi , [ str_hello ]
    mov rdx , str_hello_len
    syscall
    call fn_Exit

