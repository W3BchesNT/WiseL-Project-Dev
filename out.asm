format PE64 Console
entry start

include 'win64w.inc'

section '.text' code readable executable

start:
    mov rax , 1
    mov rbx , 2
    mov rbx , rax
    sub rsp, 8
    and rsp, -16
    invoke ExitProcess, 0

section '.idata' import data readable writeable
    library kernel32, 'KERNEL32.DLL'
    import kernel32, \
           ExitProcess, 'ExitProcess'
