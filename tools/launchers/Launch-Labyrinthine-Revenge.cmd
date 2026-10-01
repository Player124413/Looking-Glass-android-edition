@echo off
setlocal
call "%~dp0..\..\Launch.cmd" --map hedge3 --entry hedge3_start1 --save-dir "%~dp0..\..\private\hedge3-playtest-saves" %*
