@echo off
if exist "%~dp0..\..\private\jabberwock\Play-Lair.cmd" (
    call "%~dp0..\..\private\jabberwock\Play-Lair.cmd" %*
) else (
    call "%~dp0..\..\Launch.cmd" --map jlair2 %*
)
