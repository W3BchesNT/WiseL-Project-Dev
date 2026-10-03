include 'extension/format.inc'
include 'extension/wir_ll.inc'

set_build ARM64, Linux, Console

entry start

; .text .code
code_block
  start:
	sys_prolog

	sys_load  t0_64, result
	sys_const t1_64, 10
	sys_mul   t2_64, t0_64, t1_64
	sys_store result, t2_64

	match =ARM64, CURRENT_ARCH
		sys_call 64, 1, str_hello, str_hello_len
		sys_call 93, 0
	else match =X64, CURRENT_ARCH
		sys_call 1, 1, str_hello, str_hello_len
		sys_call 60, 0
	end match

; .data
data_block
	sys_var    hStdOut, i64
	sys_var    bytesWritten, i32
	sys_var    result, i64, 5
	sys_string str_hello, <'WIR Language completely unified on Linux!', 13, 10, 0>

; WILL BE IGNORED IN LINUX
import_block
	library kernel32, 'KERNEL32.DLL'
	import kernel32, GetStdHandle, 'GetStdHandle', WriteConsoleA, 'WriteConsoleA', ExitProcess, 'ExitProcess'
