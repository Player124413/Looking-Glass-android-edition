@echo off
setlocal
if exist "%~dp0..\..\private\rchess1\Play.cmd" (
    call "%~dp0..\..\private\rchess1\Play.cmd" %*
    exit /b
)
call "%~dp0..\..\Launch.cmd" --map rchess1 %*
