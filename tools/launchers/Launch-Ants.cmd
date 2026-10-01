@echo off
if exist "%~dp0..\..\private\ants\Play-Ants.cmd" (
    call "%~dp0..\..\private\ants\Play-Ants.cmd" %*
) else (
    call "%~dp0..\..\Launch.cmd" --map potears1 %*
)
