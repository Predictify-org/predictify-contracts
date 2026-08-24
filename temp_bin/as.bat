@echo off
setlocal enabledelayedexpansion
set "args="
for %%a in (%*) do (
    set "skip="
    if "%%a"=="--64" set "skip=1"
    if "%%a"=="--32" set "skip=1"
    if "%%a"=="--no-leading-underscore" set "skip=1"
    if not defined skip (
        set "args=!args! %%a"
    )
)
"C:\Users\NEW USER\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained\x86_64-w64-mingw32-gcc.exe" -c !args!
