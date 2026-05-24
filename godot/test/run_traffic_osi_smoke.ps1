# Build the extension, register it for a headless run, and execute the
# OSI-driven traffic-light smoke test (mock OSI stream -> receiver -> converter
# -> traffic-light heads). Exits non-zero on failure so it can gate CI.
#
# Usage (from anywhere):
#   pwsh godot/test/run_traffic_osi_smoke.ps1 [-Godot <path-to-godot_console.exe>]
param(
    # Godot 4.6 console binary. Override with -Godot if it lives elsewhere.
    [string]$Godot = "$PSScriptRoot/../../temp/Godot_v4.6.3-stable_win64.exe/Godot_v4.6.3-stable_win64_console.exe"
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path "$PSScriptRoot/../.."
$proj = Resolve-Path "$PSScriptRoot/.."   # the godot/ project

Write-Host "==> cargo build -p godot-osi"
Push-Location $repoRoot
try {
    cargo build -p godot-osi
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
}
finally { Pop-Location }

$dll = Join-Path $repoRoot "target/debug/godot_osi.dll"
if (-not (Test-Path $dll)) { throw "dll not found: $dll" }

# Register the GDExtension without a prior editor import (headless only loads
# extensions listed here). .godot/ is gitignored.
New-Item -ItemType Directory -Force (Join-Path $proj ".godot") | Out-Null
Set-Content -Path (Join-Path $proj ".godot/extension_list.cfg") `
    -Value "res://addons/godot_osi/godot_osi.gdextension"

if (-not (Test-Path $Godot)) { throw "Godot binary not found: $Godot (pass -Godot <path>)" }

Write-Host "==> running headless OSI traffic-light smoke test"
& $Godot --headless --path $proj --script "res://test/traffic_osi_smoke.gd"
$code = $LASTEXITCODE
if ($code -eq 0) { Write-Host "TRAFFIC OSI OK" } else { Write-Host "TRAFFIC OSI FAILED ($code)" }
exit $code
