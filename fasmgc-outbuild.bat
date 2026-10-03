@echo off
setlocal

cargo run

cd /d "%~dp0"

set "INCLUDE=.\kernel\FASM\fasmg\packages\x86\include"

kernel\FASM\fasmg\fasmg.exe out.asm main.exe

if %errorlevel% neq 0 (
    exit /b 1
)
main.exe

endlocal
