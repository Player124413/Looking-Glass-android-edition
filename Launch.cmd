@echo off
setlocal
pushd "%~dp0"
if exist "looking-glass.exe" (
    if "%~1"=="" if exist "tools\windows_setup.ps1" (
        powershell.exe -NoLogo -NoProfile -STA -WindowStyle Hidden -ExecutionPolicy Bypass -File "tools\windows_setup.ps1" -Mode Play
        goto finished
    )
    "looking-glass.exe" %*
    goto finished
)
if exist "private\playtest\Play-Latest.cmd" (
    rem Some older playtest builds include the final map but lack its mission controller.
    rem Keep ordinary map selection playable when such a build is still selected.
    if exist "private\heart-of-darkness\looking-glass.exe" (
        call "private\playtest\Play-Latest.cmd" --help 2>nul | findstr /L /C:"--qlair-check" >nul
        if errorlevel 1 (
            "private\heart-of-darkness\looking-glass.exe" --save-dir "%CD%\private\playtest-saves" %*
            goto finished
        )
    )
    call "private\playtest\Play-Latest.cmd" %*
    goto finished
)
if exist "private\playtest\looking-glass.exe" (
    "private\playtest\looking-glass.exe" --save-dir "%CD%\private\playtest-saves" %*
    goto finished
)
if not exist "target\release\looking-glass.exe" (
    where cargo >nul 2>nul
    if errorlevel 1 (
        echo This source folder needs Rust to build the game.
        echo A Windows release ZIP includes looking-glass.exe and does not need Rust.
        echo See docs\INSTALL.md for setup and build instructions.
        pause
        popd
        exit /b 1
    )
    cargo build --release --locked
    if errorlevel 1 (
        pause
        popd
        exit /b 1
    )
)
"target\release\looking-glass.exe" %*
:finished
if errorlevel 1 pause
popd
