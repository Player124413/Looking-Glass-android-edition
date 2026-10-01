@echo off
setlocal
call "%~dp0..\..\Launch.cmd" --map funhouse --entry funhouse_start1 --save-dir "%~dp0..\..\private\funhouse-playtest-saves" %*
