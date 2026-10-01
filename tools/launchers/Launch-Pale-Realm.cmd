@echo off
setlocal
call "%~dp0..\..\Launch.cmd" --map wchess1 --entry wchess1_start1 --save-dir "%~dp0..\..\private\wchess1-playtest-saves" %*
