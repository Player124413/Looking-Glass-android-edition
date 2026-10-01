@echo off
setlocal
call "%~dp0..\..\Launch.cmd" --map garden4 --entry garden4_start1 --save-dir "%~dp0..\..\private\garden4-playtest-saves" %*
