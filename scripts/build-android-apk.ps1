# Assemble debug APK and optionally install to running emulator/device.
# Docs: docs/tech/ANDROID_DUAL_PLATFORM.md

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$Android = Join-Path $Root "apps\android"
Set-Location $Android

$sdk = "$env:LOCALAPPDATA\Android\Sdk"
$env:ANDROID_HOME = $sdk
$env:ANDROID_SDK_ROOT = $sdk
$env:JAVA_HOME = (Get-Command java -ErrorAction SilentlyContinue | ForEach-Object { Split-Path (Split-Path $_.Source) })
# Prefer JDK 17/21 if present
$jdkCandidates = @(
    "C:\Program Files\Java\jdk-21.0.11",
    "C:\Program Files\Android\Android Studio\jbr",
    "C:\Program Files\Eclipse Adoptium\jdk-21.0.11*"
)
foreach ($c in $jdkCandidates) {
    $resolved = Get-Item $c -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($resolved -and (Test-Path (Join-Path $resolved.FullName "bin\java.exe"))) {
        $env:JAVA_HOME = $resolved.FullName
        break
    }
}
$env:Path = "$env:JAVA_HOME\bin;$sdk\platform-tools;$env:Path"

Write-Host "JAVA_HOME=$env:JAVA_HOME" -ForegroundColor Cyan
Write-Host "=== gradlew assembleDebug ===" -ForegroundColor Cyan
& .\gradlew.bat --no-daemon assembleDebug
if ($LASTEXITCODE -ne 0) { throw "gradle failed: $LASTEXITCODE" }

$apk = Join-Path $Android "app\build\outputs\apk\debug\app-debug.apk"
if (-not (Test-Path $apk)) { throw "APK missing: $apk" }
Write-Host "APK=$apk" -ForegroundColor Green

$adb = Join-Path $sdk "platform-tools\adb.exe"
Write-Host "=== adb devices ===" -ForegroundColor Cyan
& $adb devices

# 默认只装包、不 am start（真机操控归主人；NORMS / 交接约定）
$launch = $env:AUTOMADOKA_ADB_LAUNCH -eq "1"
$devices = & $adb devices | Select-String "\tdevice$"
if ($devices) {
    Write-Host "=== adb install -r ===" -ForegroundColor Cyan
    & $adb install -r $apk
    if ($LASTEXITCODE -ne 0) { throw "adb install failed" }
    if ($launch) {
        Write-Host "=== launch MainActivity (AUTOMADOKA_ADB_LAUNCH=1) ===" -ForegroundColor Cyan
        & $adb shell am start -n com.rustmadoka.android.debug/com.rustmadoka.android.MainActivity
        Write-Host "INSTALL_LAUNCH_OK" -ForegroundColor Green
    } else {
        Write-Host "INSTALL_OK (no launch; set AUTOMADOKA_ADB_LAUNCH=1 to auto-start)" -ForegroundColor Green
    }
} else {
    Write-Host "No adb device; APK built only. Authorize device then re-run." -ForegroundColor Yellow
    Write-Host "APK_ONLY_OK" -ForegroundColor Green
}
