macro set_build? arch*, os*, type*
    define CURRENT_ARCH arch
    match =Linux, os
        define CURRENT_OS 'LINUX'
        include 'format/elfexe.inc'
        ELF.Settings.Class = 2
        ELF.Settings.Type = ET_DYN
        ELF.Settings.LoadHeaders = 1
        match =ARM64, arch
            ELF.Settings.Machine = EM_AARCH64
            include '../../aarch64/iset/aarch64.inc'
            define t0_64 x0
            define t0_32 w0
            define t1_64 x1
            define t1_32 w1
            define t2_64 x2
            define t2_32 w2
            define t3_64 x3
            define t3_32 w3
        else match =X64, arch
            ELF.Settings.Machine = EM_X86_64
            include 'cpu/x64.inc'
            define t0_64 rax
            define t0_32 eax
            define t1_64 rcx
            define t1_32 ecx
            define t2_64 rdx
            define t2_32 edx
            define t3_64 rbx
            define t3_32 ebx
        end match
    else match =Windows, os
        define CURRENT_OS 'WINDOWS'
        include 'format/format.inc'
        match =Console, type
            format PE64 console
        else match =GUI, type
            format PE64 GUI
        end match
        include 'macro/import64.inc'
        ; Убран дубликат proc64.inc, так как используется win64w.inc в out.asm
        define t0_64 rax
        define t0_32 eax
        define t1_64 rcx
        define t1_32 ecx
        define t2_64 rdx
        define t2_32 edx
        define t3_64 rbx
        define t3_32 ebx
    end match
end macro

macro code_block
    match ='WINDOWS', CURRENT_OS
        section '.text' code readable executable
    else match ='LINUX', CURRENT_OS
        segment readable executable
        match =X64, CURRENT_ARCH
            use64
        end match
    end match
end macro

macro data_block
    match ='WINDOWS', CURRENT_OS
        section '.data' data readable writeable
    else match ='LINUX', CURRENT_OS
        segment readable writeable
    end match
end macro

macro import_block
    match ='WINDOWS', CURRENT_OS
        section '.idata' import data readable writeable
    else match ='LINUX', CURRENT_OS
        macro library? dummy&
        end macro
        macro import? dummy&
        end macro
    end match
end macro