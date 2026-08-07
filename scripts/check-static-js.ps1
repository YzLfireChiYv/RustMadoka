# ASCII-only: balance braces inside static/index.html <script>
# Exit 0 ok; 1 fail. Use after editing SPA.
$ErrorActionPreference = "Stop"
# This file lives in <repo>/scripts/
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$htmlPath = Join-Path $root "crates\rustmadoka-app\static\index.html"
if (-not (Test-Path $htmlPath)) {
  Write-Error "missing $htmlPath"
  exit 1
}
$c = Get-Content -LiteralPath $htmlPath -Raw -Encoding UTF8
if ($c -notmatch '(?s)<script>(.*)</script>') {
  Write-Error "no script block"
  exit 1
}
$script = $Matches[1]
$open = ([regex]::Matches($script, '\{')).Count
$close = ([regex]::Matches($script, '\}')).Count
Write-Host "braces open=$open close=$close"
if ($open -ne $close) {
  Write-Error "brace mismatch delta=$($open-$close)"
  exit 1
}
# Footgun: stray try after let following function end
if ($script -match '}\s*let \w+[^;]*;\s*try\s*\{') {
  Write-Error "suspicious pattern: } let x; try { (possible truncated function)"
  exit 1
}
if ($script -notmatch 'async function init') {
  Write-Error "missing init"
  exit 1
}
Write-Host "static js smoke: OK"
exit 0
