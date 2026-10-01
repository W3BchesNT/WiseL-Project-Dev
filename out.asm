format PE64 Console
entry start

include 'win64w.inc'

section '.data' data readable writeable
    hStdOut dq ?
    bytesWritten dd ?
    str_hello db 'Hello, World!' , 13 , 10 , 0
    hello_len = $ - str_hello - 1

section '.text' code readable executable

start:
    sub rsp , 8
    and rsp , -16
    sub rsp , 40
    
    invoke GetStdHandle , -11
    mov [ hStdOut ] , rax
    
    invoke WriteConsoleA , [ hStdOut ] , str_hello , hello_len , bytesWritten , 0
    
    invoke ExitProcess , 0
    sub rsp, 8
    and rsp, -16
    invoke ExitProcess, 0

section '.idata' import data readable writeable
    library kernel32, 'KERNEL32.DLL'
    import kernel32, \
           GetStdHandle, 'GetStdHandle', \
           WriteConsoleA, 'WriteConsoleA', \
           ExitProcess, 'ExitProcess'
