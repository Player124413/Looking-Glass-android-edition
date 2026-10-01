@echo off
setlocal
call "%~dp0..\..\Launch.cmd" --map garden2 --save-dir "%~dp0..\..\private\garden2-playtest-saves" %*
