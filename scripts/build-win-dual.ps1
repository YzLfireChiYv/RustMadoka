# UTF-8 with BOM for WinPS 5.1
# Build Windows dual packages:
#   RustMadoka.exe       — ordinary (no wire_record)
#   RustMadoka_debug.exe — dev (feature wire_record)
# Shared data folder: RustMadoka_data/
# Docs: docs/PLAN_RUSTMADOKA_FULL_REWRITE.md · docs/HANDOFF.md

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $root

$vs = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if (-not (Test-Path $vs)) {
    Write-Error "vcvars64.bat not found: $vs"
}

function Invoke-ReleaseBuild([string[]]$CargoArgs) {
    $argLine = ($CargoArgs | ForEach-Object { if ($_ -match '\s') { "`"$_`"" } else { $_ } }) -join ' '
    cmd /c "`"$vs`" && cargo build -p rustmadoka-app --release $argLine"
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed: $argLine" }
}

Write-Host "==> ordinary (no wire_record)"
Invoke-ReleaseBuild @()
# bin name is RustMadoka (Cargo.toml [[bin]] name)
if (Test-Path "target\release\RustMadoka.exe") {
    Copy-Item -Force "target\release\RustMadoka.exe" "RustMadoka.exe"
} elseif (Test-Path "target\release\rustmadoka.exe") {
    Copy-Item -Force "target\release\rustmadoka.exe" "RustMadoka.exe"
} else {
    throw "release binary not found (expected RustMadoka.exe)"
}

Write-Host "==> debug (wire_record)"
Invoke-ReleaseBuild @("--features", "wire_record")
if (Test-Path "target\release\RustMadoka.exe") {
    Copy-Item -Force "target\release\RustMadoka.exe" "RustMadoka_debug.exe"
} elseif (Test-Path "target\release\rustmadoka.exe") {
    Copy-Item -Force "target\release\rustmadoka.exe" "RustMadoka_debug.exe"
} else {
    throw "debug binary not found"
}

Write-Host "OK:"
Get-Item "RustMadoka.exe", "RustMadoka_debug.exe" | Format-Table Name, Length, LastWriteTime
