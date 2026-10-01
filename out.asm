format ELF64 executable
entry start

segment readable writeable
    str_hello db 'Hello, World!' , 10
    hello_len = $ - str_hello

segment readable executable

start:
    mov rax , 1
    mov rdi , 1
    lea rsi , [ str_hello ]
    mov rdx , hello_len
    syscall
    
    mov rax , 60
    mov rdi , 0
    syscall
