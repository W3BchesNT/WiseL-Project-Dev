include 'kernel/FASM/fasmg/extension/format.inc'
include 'kernel/FASM/fasmg/extension/wir_ll.inc'

set_build X64, Windows, Console

entry start

include 'win64w.inc'

data_block
    hStdOut dq ?
    bytesWritten dd ?
    str_hello db 'Hello, World!' , 13 , 10 , 0
    hello_len = $ - str_hello - 1

code_block

start:

    sys_prolog
    invoke GetStdHandle , -11
    sys_store hStdOut , t0_64
    sys_load t0_64 , hStdOut
    invoke WriteConsoleA , t0_64 , str_hello , hello_len , bytesWritten , 0
    invoke ExitProcess , 0

import_block
    library kernel32, 'KERNEL32.DLL'
    import kernel32, \
           GetStdHandle, 'GetStdHandle', \
           WriteConsoleA, 'WriteConsoleA', \
           ExitProcess, 'ExitProcess'
