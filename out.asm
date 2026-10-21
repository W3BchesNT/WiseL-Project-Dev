format PE64 Console
entry start


section '.data' data readable writeable
    wisel_v1 dd ?
    str_hi db 'H','e','l','l','o',',',' ','W','o','r','l','d','!',0
    str_hi_len dd 13
    str_hru db 'H','o','w',' ','a','r','e',' ','y','o','u','?',0
    str_hru_len dd 12
    str_new db 'W','h','a','t',' ','a','r','e',' ','y','o','u',' ','d','o','i','n','g','?',0
    str_new_len dd 19
    str_by db 'B','y','e',',',' ','W','o','r','l','d','!',0
    str_by_len dd 11

section '.text' code readable executable
start:
    mov rcx, 0
    call [ExitProcess]

section '.idata' import data readable writeable
    library kernel32,'KERNEL32.DLL'
    import kernel32,\
        GetStdHandle,'GetStdHandle',\
        WriteConsoleA,'WriteConsoleA',\
        ExitProcess,'ExitProcess'
