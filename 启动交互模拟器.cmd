@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\simulate-ui.ps1"
if errorlevel 1 pause
