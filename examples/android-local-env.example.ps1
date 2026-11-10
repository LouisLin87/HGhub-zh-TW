# 複製到 .tools/android-local-env.ps1，再依自己的安裝位置修改。
# Android 原生工具與建置目錄請使用 ASCII 英文路徑。
$toolRoot = 'C:\Dev\HGhubTools\android-tools'
$projectRoot = Split-Path -Parent $PSScriptRoot
$env:JAVA_HOME = Join-Path $toolRoot 'jdk-17.0.21+9'
$env:ANDROID_HOME = Join-Path $toolRoot 'sdk'
$env:ANDROID_SDK_ROOT = $env:ANDROID_HOME
$env:NDK_HOME = Join-Path $env:ANDROID_HOME 'ndk/28.2.13676358'
$env:ANDROID_USER_HOME = Join-Path $toolRoot 'android-user'
$env:GRADLE_USER_HOME = Join-Path $toolRoot 'gradle-home'
$env:RUSTUP_HOME = Join-Path $toolRoot 'rustup'
$env:CARGO_HOME = Join-Path $toolRoot 'cargo'
$env:HGHUB_ANDROID_BUILD_ROOT = 'C:\Dev\HGhubBuild\android'
$env:HGHUB_ANDROID_OUTPUT_ROOT = Join-Path $projectRoot 'artifacts/android'
$javaUserHome = $env:ANDROID_USER_HOME.Replace('\','/')
$env:JAVA_TOOL_OPTIONS = '-Duser.home=' + $javaUserHome
$env:Path = "$env:JAVA_HOME\bin;$env:ANDROID_HOME\platform-tools;$env:CARGO_HOME\bin;$env:Path"
