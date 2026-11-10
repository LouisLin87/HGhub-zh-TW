@echo off
setlocal
cd /d "%~dp0"
call "%~dp0hghub.cmd" exec powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\build-android-local.ps1" %*
exit /b %errorlevel%
