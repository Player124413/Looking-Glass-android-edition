@echo off
setlocal
call "%~dp0..\..\Launch.cmd" --map garden3 --save-dir "%~dp0..\..\private\garden3-playtest-saves" %*
