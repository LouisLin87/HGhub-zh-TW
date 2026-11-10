param([string]$ProjectRoot = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
$outputRoot = Join-Path $ProjectRoot ('test-results/startup-bootstrap/' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null
$utf8 = New-Object Text.UTF8Encoding($true)
function Assert-True($Condition,[string]$Message) { if (-not $Condition) { throw $Message } }
function New-Fixture([string]$Name) {
    $root = Join-Path $outputRoot ($Name + ' 空白路徑')
    New-Item -ItemType Directory -Path (Join-Path $root 'scripts') -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $ProjectRoot 'hghub.cmd') -Destination (Join-Path $root 'hghub.cmd')
    $bootstrap = Join-Path $ProjectRoot 'scripts/bootstrap-local.ps1'
    if (Test-Path -LiteralPath $bootstrap) { Copy-Item -LiteralPath $bootstrap -Destination (Join-Path $root 'scripts/bootstrap-local.ps1') }
    [IO.File]::WriteAllText((Join-Path $root 'pnpm-lock.yaml'),'lockfileVersion: 9.0',$utf8)
    $frontend = @'
$root = Split-Path -Parent $PSScriptRoot
Add-Content -LiteralPath (Join-Path $root 'events.txt') -Value 'setup:frontend'
if (Test-Path -LiteralPath (Join-Path $root 'fail-frontend')) { throw '模擬前端初始化失敗' }
foreach ($folder in @('.tools/node-v22.23.4-win-x64','.tools/pnpm/node_modules/.bin','node_modules')) { New-Item -ItemType Directory -Path (Join-Path $root $folder) -Force | Out-Null }
[IO.File]::WriteAllBytes((Join-Path $root '.tools/node-v22.23.4-win-x64/node.exe'),[byte[]]@())
[IO.File]::WriteAllText((Join-Path $root '.tools/pnpm/node_modules/.bin/pnpm.cmd'),'@echo off' + "`r`n" + '>>"%CD%\events.txt" echo pnpm:%*' + "`r`n" + 'exit /b 0' + "`r`n")
[IO.File]::WriteAllText((Join-Path $root 'node_modules/.modules.yaml'),'fixture: true')
'@
    $native = @'
$root = Split-Path -Parent $PSScriptRoot
Add-Content -LiteralPath (Join-Path $root 'events.txt') -Value 'setup:native'
if (Test-Path -LiteralPath (Join-Path $root 'fail-native')) { throw '模擬原生初始化失敗' }
New-Item -ItemType Directory -Path (Join-Path $root '.tools/cargo/bin') -Force | Out-Null
foreach ($name in @('rustc.exe','cargo.exe')) { [IO.File]::WriteAllBytes((Join-Path $root ('.tools/cargo/bin/' + $name)),[byte[]]@()) }
'@
    [IO.File]::WriteAllText((Join-Path $root 'scripts/setup-local.ps1'),$frontend,$utf8)
    [IO.File]::WriteAllText((Join-Path $root 'scripts/setup-native.ps1'),$native,$utf8)
    return $root
}
function Invoke-Fixture([string]$Root,[string]$Command = '') {
    $previous = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        $output = & $env:ComSpec /d /c ('call "' + (Join-Path $Root 'hghub.cmd') + '" ' + $Command + ' <NUL') 2>&1
        $status = $LASTEXITCODE
    } finally { $ErrorActionPreference = $previous }
    return @{ exitCode = $status; output = ($output | Out-String); events = @(Get-Content -LiteralPath (Join-Path $Root 'events.txt') -ErrorAction SilentlyContinue) }
}
$fixture = New-Fixture '首次啟動'
$first = Invoke-Fixture $fixture
Assert-True ($first.exitCode -eq 0) ('乾淨原始碼應自動初始化並啟動：' + $first.output)
Assert-True (($first.events -join '|') -eq 'setup:frontend|setup:native|pnpm:app:dev') '首次啟動應完成環境再啟動 App'
$second = Invoke-Fixture $fixture
Assert-True (($second.events | Where-Object { $_ -like 'setup:*' }).Count -eq 2) '再次啟動不應重複安裝工具'
$forwarded = Invoke-Fixture $fixture 'exec node --version'
Assert-True ($forwarded.events[-1] -eq 'pnpm:exec node --version') '命令及參數應完整傳遞'
Remove-Item -LiteralPath (Join-Path $fixture 'node_modules/.modules.yaml')
$repaired = Invoke-Fixture $fixture 'build'
Assert-True ($repaired.exitCode -eq 0 -and $repaired.events[-2] -eq 'setup:frontend' -and $repaired.events[-1] -eq 'pnpm:build') '缺少相依套件時應先恢復環境再執行命令'
$failed = New-Fixture '設定失敗'
[IO.File]::WriteAllText((Join-Path $failed 'fail-native'),'true')
$failure = Invoke-Fixture $failed
Assert-True ($failure.exitCode -ne 0) '初始化失敗必須保留失敗狀態'
Assert-True (-not ($failure.events | Where-Object { $_ -like 'pnpm:*' })) '初始化失敗後不應啟動 App'
@{ passed = @('乾淨原始碼首次啟動','再次啟動不重裝','命令參數傳遞','缺少相依套件時恢復','初始化失敗不啟動 App'); fixtureOnly = $true; installedRealTools = $false; output = $outputRoot } | ConvertTo-Json -Depth 3
