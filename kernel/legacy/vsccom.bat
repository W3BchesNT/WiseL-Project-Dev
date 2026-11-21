@echo off
chcp 65001 > nul
cls

call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat" > nul
cl /nologo /EHsc /O1 /GL /MD kernel\main.cpp kernel\lexer.cpp kernel\parser.cpp kernel\codegen.cpp /Fewiselc.exe /link /INCREMENTAL:NO /OPT:REF /OPT:ICF > nul

if %errorlevel% neq 0 (
	echo.
	echo [ERROR] The Build Failed!
	pause
	exit /b %errorlevel%
)

del /q *.obj > nul 2>&1

echo Windows Build Done!
echo PE binary: %cd%\wiselc.exe
pause