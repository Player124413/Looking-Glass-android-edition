@echo off
if exist "%~dp0..\..\private\jabberwock\Play-Royal-Rage.cmd" (
    call "%~dp0..\..\private\jabberwock\Play-Royal-Rage.cmd" %*
) else (
    call "%~dp0..\..\Launch.cmd" --map grounds1 %*
)
