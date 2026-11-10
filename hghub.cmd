@echo off
chcp 65001 >nul
setlocal
cd /d "%~dp0"
if /i "%~1"=="setup" goto setup
if /i "%~1"=="setup-native" goto native
if "%~1"=="" goto prepare_native
if /i "%~1"=="app:dev" goto prepare_native
if /i "%~1"=="app:build" goto prepare_native
goto prepare_frontend
:prepare_native
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\bootstrap-local.ps1" -Native
if errorlevel 1 goto failed
goto tools
:prepare_frontend
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\bootstrap-local.ps1"
if errorlevel 1 goto failed
:tools
set "CARGO_HOME=%~dp0.tools\cargo"
set "RUSTUP_HOME=%~dp0.tools\rustup"
set "PATH=%~dp0.tools\node-v22.23.4-win-x64;%~dp0.tools\pnpm\node_modules\.bin;%CARGO_HOME%\bin;%PATH%"
set "npm_config_cache=%~dp0.tools\npm-cache"
set "npm_config_store_dir=%~dp0.pnpm-store"
if /i "%~1"=="shell" goto shell
if /i "%~1"=="versions" goto versions
if not "%~1"=="" goto run
echo 環境已就緒，正在啟動 HGhub。首次原生編譯需要較長時間，請保持視窗開啟。
call pnpm.cmd app:dev
if errorlevel 1 goto failed
exit /b 0
:run
call pnpm.cmd %*
if errorlevel 1 goto failed
exit /b 0
:setup
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\setup-local.ps1"
if errorlevel 1 goto failed
exit /b 0
:native
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\setup-native.ps1"
if errorlevel 1 goto failed
exit /b 0
:shell
echo 此終端機已載入專案工具，輸入 exit 可關閉。
cmd.exe /k
exit /b %errorlevel%
:versions
node --version
call pnpm.cmd --version
if exist "%CARGO_HOME%\bin\rustc.exe" rustc --version
if exist "%CARGO_HOME%\bin\cargo.exe" cargo --version
exit /b %errorlevel%
:failed
set "startup_status=%errorlevel%"
echo.
echo 設定或啟動未完成，請保留上方錯誤文字以便排查。
if "%~1"=="" pause
exit /b %startup_status%
