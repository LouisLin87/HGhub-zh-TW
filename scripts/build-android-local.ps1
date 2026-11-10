[CmdletBinding()]
param([ValidateSet('build','devices')][string]$Action = 'build')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$environmentFile = Join-Path $projectRoot '.tools/android-local-env.ps1'
if (-not (Test-Path -LiteralPath $environmentFile)) { throw 'Android tool configuration is missing. See docs/android-local-development.zh-TW.md.' }
. $environmentFile
if ($Action -eq 'devices') { & (Join-Path $env:ANDROID_HOME 'platform-tools/adb.exe') devices -l; exit $LASTEXITCODE }

function Get-Sha256([string]$Path) {
    $stream = [IO.File]::OpenRead($Path)
    $sha = [Security.Cryptography.SHA256]::Create()
    try { return [BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-','').ToLowerInvariant() }
    finally { $sha.Dispose(); $stream.Dispose() }
}

# Keep the local test certificate so subsequent APKs can update the same app.
$debugKey = Join-Path $env:ANDROID_USER_HOME 'debug.keystore'
$keyBackup = Join-Path $projectRoot '.tools/android-signing/debug.keystore'
if (Test-Path -LiteralPath $keyBackup) {
    if (Test-Path -LiteralPath $debugKey) {
        if ((Get-Sha256 $keyBackup) -ne (Get-Sha256 $debugKey)) { throw 'Android debug signing keys differ. Preserve the original key before building an update.' }
    } else {
        New-Item -ItemType Directory -Path $env:ANDROID_USER_HOME -Force | Out-Null
        Copy-Item -LiteralPath $keyBackup -Destination $debugKey
    }
}

# Build in an ASCII path because Android native tools on Windows do not reliably support Unicode project paths.
$buildRoot = $env:HGHUB_ANDROID_BUILD_ROOT
if (-not $buildRoot -or $buildRoot -match '[^\x00-\x7f]') { throw 'Use an ASCII Android build path.' }
$buildRoot = [IO.Path]::GetFullPath($buildRoot)
New-Item -ItemType Directory -Path $buildRoot -Force | Out-Null
Push-Location -LiteralPath $projectRoot
try {
    $files = New-Object 'Collections.Generic.List[string]'
    $directories = New-Object 'Collections.Generic.Stack[string]'
    $directories.Push($projectRoot)
    $excluded = @('node_modules','.tools','.pnpm-store','.git','.vscode','.idea','dist','dist-ssr','test-results','work','outputs','android-artifacts')
    while ($directories.Count) {
        foreach ($item in Get-ChildItem -LiteralPath $directories.Pop() -Force) {
            $relative = $item.FullName.Substring($projectRoot.Length + 1)
            if ($item.PSIsContainer) {
                if ($item.Name -in $excluded -or $relative -in @('src-tauri\target','src-tauri\gen') -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) { continue }
                $directories.Push($item.FullName)
            } elseif ($item.Name -notmatch '(^\.env|\.local$|\.log$|\.keystore$|\.jks$|\.pem$|\.p12$|\.key$)') {
                $files.Add($relative)
            }
        }
    }
    foreach ($file in $files) {
        $destination = [IO.Path]::GetFullPath((Join-Path $buildRoot $file))
        if (-not $destination.StartsWith($buildRoot + '\',[StringComparison]::OrdinalIgnoreCase)) { throw 'Source destination escaped build directory.' }
        New-Item -ItemType Directory -Path (Split-Path -Parent $destination) -Force | Out-Null
        Copy-Item -LiteralPath (Join-Path $projectRoot $file) -Destination $destination -Force
    }
} finally { Pop-Location }

$modules = Join-Path $buildRoot 'node_modules'
if (-not (Test-Path -LiteralPath $modules)) {
    New-Item -ItemType Junction -Path $modules -Target (Join-Path $projectRoot 'node_modules') | Out-Null
}
Push-Location -LiteralPath $buildRoot
try {
    if (-not (Test-Path -LiteralPath 'src-tauri/gen/android/gradlew.bat')) {
        & pnpm.cmd tauri android init --ci --skip-targets-install
        if ($LASTEXITCODE -ne 0) { throw 'Android initialization failed.' }
    }
    $buildLog = Join-Path $buildRoot 'android-native-build.log'
    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    & pnpm.cmd tauri android build --debug --apk --target aarch64 --ci *> $buildLog
    $nativeExit = $LASTEXITCODE
    $ErrorActionPreference = $previousPreference
    $nativeOutput = Get-Content -LiteralPath $buildLog -Raw
    if ($nativeExit -ne 0) {
        # Tauri has already completed the Rust build at this specific failure.
        # Copy the resulting library and package it with Gradle, rather than
        # requiring Windows' global symbolic-link privilege/developer mode.
        if ($nativeOutput -notmatch 'Creation symbolic link is not allowed' -or $nativeOutput -notmatch 'Finished.*dev.*profile') {
            Get-Content -LiteralPath $buildLog -Tail 25
            throw 'Android native build failed; inspect android-native-build.log.'
        }
        Write-Host 'Native ARM64 build passed. Packaging with a copied JNI library (Windows symlink fallback).'
        $library = Join-Path $buildRoot 'src-tauri/target/aarch64-linux-android/debug/libhuangguo_client_lib.so'
        $jni = Join-Path $buildRoot 'src-tauri/gen/android/app/src/main/jniLibs/arm64-v8a'
        if (-not (Test-Path -LiteralPath $library)) { throw 'Compiled ARM64 library is missing.' }
        New-Item -ItemType Directory -Path $jni -Force | Out-Null
        $jniLibrary = Join-Path $jni 'libhuangguo_client_lib.so'
        if ((Test-Path -LiteralPath $jniLibrary) -and ((Get-Item -LiteralPath $jniLibrary -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Expected a normal JNI file; refusing to strip through a symbolic link.' }
        Copy-Item -LiteralPath $library -Destination $jniLibrary -Force
        # Strip only the distributable copy. Keep the compiled library and its
        # complete debug information in target/ for local diagnostics.
        & (Join-Path $env:NDK_HOME 'toolchains/llvm/prebuilt/windows-x86_64/bin/llvm-strip.exe') --strip-debug $jniLibrary
        if ($LASTEXITCODE -ne 0) { throw 'Removing debug information from the JNI copy failed.' }
        # Recreate this single generated APK so incremental ZIP packaging does
        # not retain unused space from an earlier full-debug native library.
        $previousApk = [IO.Path]::GetFullPath((Join-Path $buildRoot 'src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk'))
        if (-not $previousApk.StartsWith($buildRoot + '\',[StringComparison]::OrdinalIgnoreCase)) { throw 'APK output escaped build directory.' }
        if (Test-Path -LiteralPath $previousApk) { Remove-Item -LiteralPath $previousApk -Force }
        Push-Location -LiteralPath (Join-Path $buildRoot 'src-tauri/gen/android')
        try {
            & .\gradlew.bat :app:assembleUniversalDebug '-PtargetList=aarch64' '-ParchList=arm64' '-PabiList=arm64-v8a' -x :app:rustBuildUniversalDebug -x :app:rustBuildArm64Debug --no-daemon
            if ($LASTEXITCODE -ne 0) { throw 'Gradle APK packaging failed.' }
        } finally { Pop-Location }
    } else {
        Get-Content -LiteralPath $buildLog -Tail 12
    }
} finally { Pop-Location }

$apk = Join-Path $buildRoot 'src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk'
if (-not (Test-Path -LiteralPath $apk)) { throw 'The build did not produce the expected ARM64 test APK.' }
$signatureLog = Join-Path $buildRoot 'android-apk-signature.log'
$previousPreference = $ErrorActionPreference
try {
    $ErrorActionPreference = 'Continue'
    & (Join-Path $env:ANDROID_HOME 'build-tools/36.0.0/apksigner.bat') verify --verbose $apk *> $signatureLog
    $signatureExit = $LASTEXITCODE
} finally { $ErrorActionPreference = $previousPreference }
if ($signatureExit -ne 0) { throw 'APK signature verification failed; inspect android-apk-signature.log.' }
if (-not (Test-Path -LiteralPath $keyBackup)) {
    New-Item -ItemType Directory -Path (Split-Path -Parent $keyBackup) -Force | Out-Null
    Copy-Item -LiteralPath $debugKey -Destination $keyBackup
}
$outputRoot = $env:HGHUB_ANDROID_OUTPUT_ROOT
if (-not $outputRoot) { $outputRoot = Join-Path $projectRoot 'android-artifacts' }
New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null
$configuration = Get-Content -LiteralPath (Join-Path $projectRoot 'src-tauri/tauri.conf.json') -Raw | ConvertFrom-Json
$outputName = 'HGhub-local-' + $configuration.version + '-arm64-debug.apk'
$outputApk = Join-Path $outputRoot $outputName
Copy-Item -LiteralPath $apk -Destination $outputApk -Force
$digest = Get-Sha256 $outputApk
[IO.File]::WriteAllText(($outputApk + '.sha256'), ($digest + '  ' + $outputName + [Environment]::NewLine), [Text.Encoding]::ASCII)
Write-Host ('APK ready: ' + $outputApk)
Write-Host ('SHA256: ' + $digest)
