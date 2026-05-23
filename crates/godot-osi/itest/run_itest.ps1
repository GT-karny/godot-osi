# Build the converter with the `itest` feature, stage the dll into this
# project's bin/, and run the structural test headlessly. Exits non-zero on
# failure so it can gate CI.
#
# Usage (from anywhere):
#   pwsh crates/godot-osi/itest/run_itest.ps1 [-Godot <path-to-godot.exe>]
param(
    [string]$Godot = "$PSScriptRoot/../../../../../temp/Godot_v4.6.3-stable_win64.exe/Godot_v4.6.3-stable_win64_console.exe"
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path "$PSScriptRoot/../../.."
$proj = $PSScriptRoot

Write-Host "==> cargo build -p godot-osi --features itest"
Push-Location $repoRoot
try {
    cargo build -p godot-osi --features itest
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
}
finally { Pop-Location }

$dll = Join-Path $repoRoot "target/debug/godot_osi.dll"
if (-not (Test-Path $dll)) { throw "dll not found: $dll" }

New-Item -ItemType Directory -Force (Join-Path $proj "bin") | Out-Null
Copy-Item $dll (Join-Path $proj "bin/godot_osi.dll") -Force

# Register the GDExtension without needing a prior editor import: headless runs
# only load extensions listed here. (Both bin/ and .godot/ are gitignored.)
New-Item -ItemType Directory -Force (Join-Path $proj ".godot") | Out-Null
Set-Content -Path (Join-Path $proj ".godot/extension_list.cfg") -Value "res://godot_osi.gdextension"

if (-not (Test-Path $Godot)) { throw "Godot binary not found: $Godot (pass -Godot <path>)" }

Write-Host "==> running headless itest"
& $Godot --headless --path $proj --script "res://test.gd"
$code = $LASTEXITCODE
if ($code -eq 0) { Write-Host "ITEST OK" } else { Write-Host "ITEST FAILED ($code)" }
exit $code
