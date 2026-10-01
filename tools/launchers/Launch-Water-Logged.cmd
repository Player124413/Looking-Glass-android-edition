@echo off
setlocal
call "%~dp0..\..\Launch.cmd" --map tower2 --entry tower2_start1 --save-dir "%~dp0..\..\private\tower2-playtest-saves" %*
