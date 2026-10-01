@echo off
setlocal
call "%~dp0..\..\Launch.cmd" --map tower1 --entry tower1_start1 --save-dir "%~dp0..\..\private\tower1-playtest-saves" %*
