@echo off
setlocal

set "INCLUDE=kernel\FASM\fasm2\include"

if %errorlevel% neq 0 (
    echo [ERROR] WiseL compiler failed
    pause
    exit /b 1
)

kernel\FASM\fasm2\fasmg.exe -i "include 'fasm2.inc'" out.asm main.exe
if %errorlevel% neq 0 (
    echo [ERROR] FASM failed
    pause
    exit /b 1
)

echo ==========================================

echo ==========================================
pause
endlocal