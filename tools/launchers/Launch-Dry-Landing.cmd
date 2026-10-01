@echo off
setlocal
call "%~dp0..\..\Launch.cmd" --map garden1 --save-dir "%~dp0..\..\private\garden1-playtest-saves" %*
