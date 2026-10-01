@echo off
setlocal
if exist "%~dp0..\..\private\hedge1\Play.cmd" (
    call "%~dp0..\..\private\hedge1\Play.cmd" %*
    exit /b
)
call "%~dp0..\..\Launch.cmd" --map hedge1 --entry hedge1_start1 %*
