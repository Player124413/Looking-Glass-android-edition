@echo off
setlocal
if exist "%~dp0..\..\private\hatter2\Play.cmd" (
    call "%~dp0..\..\private\hatter2\Play.cmd" %*
    exit /b
)
call "%~dp0..\..\Launch.cmd" --map hatter2 %*
