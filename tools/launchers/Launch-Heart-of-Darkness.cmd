@echo off
setlocal
pushd "%~dp0..\..\"
if not exist "private\heart-of-darkness\looking-glass.exe" (
    echo The tested Heart of Darkness build is missing.
    echo See docs\QLAIR.md for build instructions.
    pause
    popd
    exit /b 1
)
"private\heart-of-darkness\looking-glass.exe" --map qlair --save-dir "%CD%\private\heart-of-darkness\saves" %*
if errorlevel 1 pause
popd
