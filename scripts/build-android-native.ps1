# Build librustmadoka_mobile.so for Android (x86_64 emulator + arm64 device)
# UTF-8 BOM recommended for PS 5.1 Chinese. Docs: docs/tech/ANDROID_DUAL_PLATFORM.md
# Opens verbose output in this console (run via Start-Process -NoExit for a visible window).

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path
$sdk = "$env:LOCALAPPDATA\Android\Sdk"
$ndk = Get-ChildItem "$sdk\ndk" -Directory | Sort-Object Name -Descending | Select-Object -First 1
if (-not $ndk) { throw "NDK not found under $sdk\ndk" }
$env:ANDROID_HOME = $sdk
$env:ANDROID_SDK_ROOT = $sdk
$env:ANDROID_NDK_HOME = $ndk.FullName
$env:NDK_HOME = $ndk.FullName

Write-Host "ROOT=$Root" -ForegroundColor Cyan
Write-Host "NDK=$($ndk.FullName)" -ForegroundColor Cyan
Write-Host "=== rustup targets ===" -ForegroundColor Cyan
rustup target add x86_64-linux-android aarch64-linux-android

if (-not (Get-Command cargo-ndk -ErrorAction SilentlyContinue)) {
    Write-Host "=== install cargo-ndk ===" -ForegroundColor Cyan
    cargo install cargo-ndk --locked
}

$jniOut = Join-Path $Root "apps\android\app\src\main\jniLibs"
New-Item -ItemType Directory -Force -Path $jniOut | Out-Null

Write-Host "=== cargo ndk build rustmadoka-mobile (release) ===" -ForegroundColor Cyan
# x86_64 = emulator automadoka_api34; arm64-v8a = real devices
cargo ndk -t x86_64 -t arm64-v8a -o $jniOut build -p rustmadoka-mobile --release
if ($LASTEXITCODE -ne 0) { throw "cargo ndk failed: $LASTEXITCODE" }

Write-Host "=== jniLibs tree ===" -ForegroundColor Green
Get-ChildItem $jniOut -Recurse -Filter "*.so" | ForEach-Object { Write-Host $_.FullName }
Write-Host "NATIVE_BUILD_OK" -ForegroundColor Green
