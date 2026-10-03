macro sys_const? dest*, val*
	mov	dest, val
end macro

macro sys_load? dest*, src_var*
	match =ARM64, CURRENT_ARCH
		adr	x16, src_var
		ldr	dest, [x16]
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
			mov	dest, qword [src_var]
		end match
	end match
end macro

macro sys_store? dest_var*, src*
	match =ARM64, CURRENT_ARCH
		adr	x16, dest_var
		str	src, [x16]
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
			mov	qword [dest_var], src
		end match
	end match
end macro

macro sys_add? dest*, src1*, src2*
	match =ARM64, CURRENT_ARCH
		add	dest, src1, src2
	else
		if ~ dest eq src1
			mov	dest, src1
		end if
		add	dest, src2
	end match
end macro

macro sys_sub? dest*, src1*, src2*
	match =ARM64, CURRENT_ARCH
		sub	dest, src1, src2
	else
		if ~ dest eq src1
			mov	dest, src1
		end if
		sub	dest, src2
	end match
end macro

macro sys_mul? dest*, src1*, src2*
	match =ARM64, CURRENT_ARCH
		mul	dest, src1, src2
	else
		if ~ dest eq src1
			mov	dest, src1
		end if
		imul	dest, src2
	end match
end macro

macro sys_call? id*, p1, p2, p3, p4
	match =ARM64, CURRENT_ARCH
		mov	x8, id
		match any, p1
			mov	x0, p1
		end match
		match any, p2
			adr	x1, p2
		end match
		match any, p3
			mov	x2, p3
		end match
		match any, p4
			mov	x3, p4
		end match
		svc	0
	else match =X64, CURRENT_ARCH
		mov	rax, id
		match any, p1
			mov	rdi, p1
		end match
		match any, p2
			lea	rsi, [p2]
		end match
		match any, p3
			mov	rdx, p3
		end match
		match any, p4
			mov	r10, p4
		end match
		syscall
	end match
end macro

macro sys_prolog local_space=0
	match ='WINDOWS', CURRENT_OS
		sub	rsp, 8
		and	rsp, -16
		sub	rsp, 40 + local_space
	end match
end macro

macro sys_var? name*, type*, val=?
	match =i64, type
		name dq val
	else match =i32, type
		name dd val
	end match
end macro

macro sys_string? name*, val*
	name db val
	name#_len = $ - name
end macro
