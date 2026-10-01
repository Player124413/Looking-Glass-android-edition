@echo off
if exist "%~dp0..\..\private\chess\Play-Chess.cmd" (
    call "%~dp0..\..\private\chess\Play-Chess.cmd" %*
) else (
    call "%~dp0..\..\Launch.cmd" --map wforest %*
)
