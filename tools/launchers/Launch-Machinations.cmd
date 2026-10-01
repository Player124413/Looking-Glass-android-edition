@echo off
setlocal
call "%~dp0..\..\Launch.cmd" --map tower3 --entry tower3_start1 --save-dir "%~dp0..\..\private\tower3-playtest-saves" %*
