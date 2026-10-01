@echo off
setlocal
if exist "%~dp0..\..\private\potears2\Play.cmd" (
    call "%~dp0..\..\private\potears2\Play.cmd" %*
    exit /b
)
call "%~dp0..\..\Launch.cmd" --map potears2 --entry potears2_start1 %*
