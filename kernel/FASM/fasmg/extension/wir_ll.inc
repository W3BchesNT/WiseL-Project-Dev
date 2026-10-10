macro WCONST? dest*, val*
    mov     dest, val
end macro

macro WLOAD? dest*, src_var*
    match =ARM64, CURRENT_ARCH
        adrp    x16, src_var
        ldr     dest, [x16, :lo12:src_var]
    else
        match =t0_32, dest
            mov dest, dword [src_var]
        else match =t1_32, dest
            mov dest, dword [src_var]
        else match =t2_32, dest
            mov dest, dword [src_var]
        else match =t3_32, dest
            mov dest, dword [src_var]
        else
            mov     dest, qword [src_var]
        end match
    end match
end macro

macro WSTORE? dest_var*, src*
    match =ARM64, CURRENT_ARCH
        adrp    x16, dest_var
        str     src, [x16, :lo12:dest_var]
    else
        match =t0_32, src
            mov dword [dest_var], src
        else match =t1_32, src
            mov dword [dest_var], src
        else match =t2_32, src
            mov dword [dest_var], src
        else match =t3_32, src
            mov dword [dest_var], src
        else
            mov     qword [dest_var], src
        end match
    end match
end macro

macro WADD? dest*, src1*, src2*
    match =ARM64, CURRENT_ARCH
        add     dest, src1, src2
    else
        if ~ dest eq src1
            mov     dest, src1
        end if
        add     dest, src2
    end match
end macro

macro WSUB? dest*, src1*, src2*
    match =ARM64, CURRENT_ARCH
        sub     dest, src1, src2
    else
        if ~ dest eq src1
            mov     dest, src1
        end if
        sub     dest, src2
    end match
end macro

macro WMUL? dest*, src1*, src2*
    match =ARM64, CURRENT_ARCH
        mul     dest, src1, src2
    else
        if ~ dest eq src1
            mov     dest, src1
        end if
        imul    dest, src2
    end match
end macro

macro WVAR? name*, type*, val=?
    match =i64, type
        name dq val
    else match =i32, type
        name dd val
    end match
end macro

macro WSTR? name, values&
    name db values
end macro

macro WSTR_LEN? name*, modifier&
    name#_len = $ - name modifier
end macro

macro LINUX_WSTR_LEN? name
    name#_len = $ - name
end macro

macro WINDOWS_WSTR_LEN? name
    name#_len = $ - name - 1
end macro

macro WLEA? dest*, src_addr*
    match =ARM64, CURRENT_ARCH
        adrp    x16, src_addr
        add     dest, x16, :lo12:src_addr
    else
        lea     dest, [src_addr]
    end match
end macro

macro WSYSCALL?
    match =ARM64, CURRENT_ARCH
        svc     0
    else
        syscall
    end match
end macro