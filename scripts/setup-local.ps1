[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$projectRoot = Split-Path -Parent $PSScriptRoot
$toolsRoot = Join-Path $projectRoot '.tools'
$nodeVersion = '22.23.4'
$pnpmVersion = '10.34.9'
$nodeFolder = "node-v$nodeVersion-win-x64"
$nodeRoot = Join-Path $toolsRoot $nodeFolder
$pnpmRoot = Join-Path $toolsRoot 'pnpm'
$pnpmBinRoot = Join-Path $pnpmRoot 'node_modules/.bin'
$downloads = Join-Path $toolsRoot 'downloads'
$nodeExe = Join-Path $nodeRoot 'node.exe'
$npmCmd = Join-Path $nodeRoot 'npm.cmd'
$pnpmCmd = Join-Path $pnpmBinRoot 'pnpm.cmd'
$previousPath = $env:Path
$previousCache = $env:npm_config_cache
$previousStore = $env:npm_config_store_dir

function Get-ArchiveHash([string]$FilePath) {
    $sha = [Security.Cryptography.SHA256]::Create()
    $stream = [IO.File]::OpenRead($FilePath)
    try { return [BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-', '') }
    finally { $stream.Dispose(); $sha.Dispose() }
}

if (-not [Environment]::Is64BitOperatingSystem) { throw 'This setup requires 64-bit Windows.' }
if (-not (Test-Path -LiteralPath (Join-Path $projectRoot 'pnpm-lock.yaml'))) {
    throw 'Run this script from the HGhub project with its original pnpm-lock.yaml.'
}

try {
    New-Item -ItemType Directory -Path $downloads -Force | Out-Null
    if (-not (Test-Path -LiteralPath $nodeExe)) {
        $archiveName = "$nodeFolder.zip"
        $archive = Join-Path $downloads $archiveName
        $baseUrl = "https://nodejs.org/dist/v$nodeVersion"
        Write-Host "Downloading Node.js $nodeVersion from nodejs.org..."
        Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/$archiveName" -OutFile $archive
        $checksumFile = Join-Path $downloads "node-v$nodeVersion-SHASUMS256.txt"
        Invoke-WebRequest -UseBasicParsing -Uri "$baseUrl/SHASUMS256.txt" -OutFile $checksumFile
        $checksums = Get-Content -LiteralPath $checksumFile -Raw
        $checksumLine = ($checksums -split "`n") | Where-Object { $_.Trim().EndsWith("  $archiveName") }
        if (@($checksumLine).Count -ne 1) { throw 'Could not find the official Node.js SHA-256 checksum.' }
        $expectedHash = ($checksumLine.Trim() -split '\s+')[0]
        $actualHash = Get-ArchiveHash $archive
        if ($actualHash -ne $expectedHash) { throw 'Node.js archive checksum verification failed.' }
        Expand-Archive -LiteralPath $archive -DestinationPath $toolsRoot -Force
        Write-Host 'Node.js SHA-256 verified.'
    }

    $env:Path = "$nodeRoot;$pnpmBinRoot;$previousPath"
    $env:npm_config_cache = Join-Path $toolsRoot 'npm-cache'
    $env:npm_config_store_dir = Join-Path $projectRoot '.pnpm-store'
    if (-not (Test-Path -LiteralPath $pnpmCmd)) {
        Write-Host "Installing pnpm $pnpmVersion in .tools/pnpm..."
        & $npmCmd install --prefix $pnpmRoot --no-audit --no-fund --ignore-scripts --package-lock=false "pnpm@$pnpmVersion"
        if ($LASTEXITCODE -ne 0) { throw "pnpm installation failed (exit $LASTEXITCODE)." }
    }
    & $nodeExe --version
    & $pnpmCmd --version
    Push-Location -LiteralPath $projectRoot
    try {
        & $pnpmCmd install --frozen-lockfile
        if ($LASTEXITCODE -ne 0) { throw "Dependency installation failed (exit $LASTEXITCODE)." }
    } finally { Pop-Location }
    Write-Host '前端環境已完成，接著可直接雙擊 hghub.cmd 啟動 App。'
} finally {
    $env:Path = $previousPath
    $env:npm_config_cache = $previousCache
    $env:npm_config_store_dir = $previousStore
}
