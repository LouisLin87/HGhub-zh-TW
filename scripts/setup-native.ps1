[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$projectRoot = Split-Path -Parent $PSScriptRoot
$toolsRoot = Join-Path $projectRoot '.tools'
$previousPath = $env:Path
$previousCargo = $env:CARGO_HOME
$previousRustup = $env:RUSTUP_HOME
$downloads = Join-Path $toolsRoot 'downloads'
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'

function Get-ArchiveHash([string]$FilePath) {
    $sha = [Security.Cryptography.SHA256]::Create()
    $stream = [IO.File]::OpenRead($FilePath)
    try { return [BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-', '') }
    finally { $stream.Dispose(); $sha.Dispose() }
}

try {
    New-Item -ItemType Directory -Path $downloads -Force | Out-Null
    $buildTools = $null
    if (Test-Path -LiteralPath $vswhere) {
        $buildTools = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    }
    if (-not $buildTools) {
        Write-Host 'Installing Microsoft C++ Build Tools and Windows SDK. Allow the Windows permission prompt if it appears.'
        & winget.exe install --id Microsoft.VisualStudio.2022.BuildTools --exact --source winget --accept-source-agreements --accept-package-agreements --silent --override '--wait --quiet --norestart --nocache --add Microsoft.VisualStudio.Workload.VCTools --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 --add Microsoft.VisualStudio.Component.Windows11SDK.26100'
        if ($LASTEXITCODE -notin @(0, 3010)) { throw "C++ Build Tools installation failed (exit $LASTEXITCODE)." }
        $buildTools = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if (-not $buildTools) { throw 'C++ Build Tools were not detected after installation.' }
    }
    Write-Host "C++ Build Tools: $buildTools"

    $env:CARGO_HOME = Join-Path $toolsRoot 'cargo'
    $env:RUSTUP_HOME = Join-Path $toolsRoot 'rustup'
    $cargoBin = Join-Path $env:CARGO_HOME 'bin'
    $env:Path = "$cargoBin;$previousPath"
    $rustc = Join-Path $cargoBin 'rustc.exe'
    if (-not (Test-Path -LiteralPath $rustc)) {
        $installer = Join-Path $downloads 'rustup-init.exe'
        $rustupUrl = 'https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe'
        Invoke-WebRequest -UseBasicParsing -Uri $rustupUrl -OutFile $installer
        $checksumFile = Join-Path $downloads 'rustup-init.exe.sha256'
        Invoke-WebRequest -UseBasicParsing -Uri "$rustupUrl.sha256" -OutFile $checksumFile
        $checksum = Get-Content -LiteralPath $checksumFile -Raw
        $expected = ($checksum.Trim() -split '\s+')[0]
        if ((Get-ArchiveHash $installer) -ne $expected) {
            throw 'Rustup SHA-256 verification failed.'
        }
        & $installer -y --no-modify-path --profile minimal --default-host x86_64-pc-windows-msvc --default-toolchain stable
        if ($LASTEXITCODE -ne 0) { throw "Rust installation failed (exit $LASTEXITCODE)." }
    }
    & $rustc --version
    if ($LASTEXITCODE -ne 0) { throw 'Rust version check failed.' }
    & (Join-Path $cargoBin 'cargo.exe') --version
    if ($LASTEXITCODE -ne 0) { throw 'Cargo version check failed.' }
    Write-Host '原生環境已完成，可直接雙擊 hghub.cmd 啟動。'
} finally {
    $env:Path = $previousPath
    $env:CARGO_HOME = $previousCargo
    $env:RUSTUP_HOME = $previousRustup
}
