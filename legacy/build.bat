@echo off
setlocal

set "INCLUDE=kernel\FASM\fasm2\include"

echo [1/4] Running WiseL compiler...
wiselc.exe
if %errorlevel% neq 0 (
    echo [ERROR] WiseL compiler failed
    pause
    exit /b 1
)

echo [2/4] Running FASM 2 (fasmg) with fasm2.inc header...

kernel\FASM\fasm2\fasmg.exe -i "include 'fasm2.inc'" out.asm main.exe
if %errorlevel% neq 0 (
    echo [ERROR] FASM 2 compilation failed
    pause
    exit /b 1
)

echo [3/4] Build successful! Running main.exe:
echo ==========================================
main.exe
echo ==========================================
pause
endlocal