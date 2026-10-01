@echo off
setlocal
call "%~dp0..\..\Launch.cmd" --map wchess2 --entry wchess2_start1 --save-dir "%~dp0..\..\private\wchess2-playtest-saves" %*
