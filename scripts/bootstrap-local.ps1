param([switch]$Native)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$toolsRoot = Join-Path $projectRoot '.tools'
try {
    $frontendReady = (Test-Path -LiteralPath (Join-Path $toolsRoot 'node-v22.23.4-win-x64/node.exe')) -and
        (Test-Path -LiteralPath (Join-Path $toolsRoot 'pnpm/node_modules/.bin/pnpm.cmd')) -and
        (Test-Path -LiteralPath (Join-Path $projectRoot 'node_modules/.modules.yaml'))
    if (-not $frontendReady) {
        Write-Host '首次啟動：正在為這份原始碼建立 Node.js、pnpm 與相依套件。' -ForegroundColor Cyan
        Write-Host '需要網路下載官方工具，首次設定可能需要幾分鐘，請保持視窗開啟。'
        & (Join-Path $PSScriptRoot 'setup-local.ps1')
        if (-not (Test-Path -LiteralPath (Join-Path $toolsRoot 'node-v22.23.4-win-x64/node.exe')) -or
            -not (Test-Path -LiteralPath (Join-Path $toolsRoot 'pnpm/node_modules/.bin/pnpm.cmd')) -or
            -not (Test-Path -LiteralPath (Join-Path $projectRoot 'node_modules/.modules.yaml'))) {
            throw '前端環境尚未完成，請檢查上方的設定錯誤。'
        }
    }
    if ($Native -and (-not (Test-Path -LiteralPath (Join-Path $toolsRoot 'cargo/bin/rustc.exe')) -or
        -not (Test-Path -LiteralPath (Join-Path $toolsRoot 'cargo/bin/cargo.exe')))) {
        Write-Host '正在確認 Windows 原生建置工具並建立 Rust 環境。' -ForegroundColor Cyan
        Write-Host '若缺少 Microsoft C++ Build Tools，系統可能要求安裝權限。'
        & (Join-Path $PSScriptRoot 'setup-native.ps1')
        if (-not (Test-Path -LiteralPath (Join-Path $toolsRoot 'cargo/bin/rustc.exe')) -or
            -not (Test-Path -LiteralPath (Join-Path $toolsRoot 'cargo/bin/cargo.exe'))) {
            throw 'Rust 環境尚未完成，請檢查上方的設定錯誤。'
        }
    }
} catch {
    Write-Host '本機環境初始化未完成，尚未啟動 App。' -ForegroundColor Red
    Write-Host $_.Exception.Message -ForegroundColor Red
    exit 1
}
