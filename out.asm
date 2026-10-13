include 'kernel/FASM/fasmg/extension/format.inc'
include 'kernel/FASM/fasmg/extension/wir_ll.inc'

set_build X64, Linux, Console
entry start

data_block
    WSTR str_hi , 'Hello, World!' , 13 , 10
    WSTR_LEN str_hi
    WSTR str_hru , 'How are you?' , 13 , 10
    WSTR_LEN str_hru
    WSTR str_new , 'What are you doing?' , 13 , 10
    WSTR_LEN str_new
    WSTR str_by , 'Bye, World!' , 13 , 10
    WSTR_LEN str_by
    stock dd 10

code_block

start:
    mov rax ,1
    mov rdi ,1
    lea rsi ,[ str_hi]
    mov rdx ,str_hi_len
    syscall
    mov rax ,1
    mov rdi ,1
    lea rsi ,[ str_hru]
    mov rdx ,str_hru_len
    syscall
    mov rax ,1
    mov rdi ,1
    lea rsi ,[ str_new]
    mov rdx ,str_new_len
    syscall
    mov eax, 1
    lea rsi ,[ str_by]
    mov rdx ,str_by_len
    syscall
    mov rax ,60
    xor rdi ,rdi
    syscall

