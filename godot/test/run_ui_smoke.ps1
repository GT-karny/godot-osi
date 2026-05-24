# Build the extension, import the project (so the runtime settings UI's
# `class_name` scripts register in the global class cache), then run the
# headless UI preset/data smoke test.
# Exits non-zero on failure so it can gate CI.
#
# Usage (from anywhere):
#   pwsh godot/test/run_ui_smoke.ps1 [-Godot <path-to-godot_console.exe>]
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

# Register the GDExtension for headless loads (.godot/ is gitignored).
New-Item -ItemType Directory -Force (Join-Path $proj ".godot") | Out-Null
Set-Content -Path (Join-Path $proj ".godot/extension_list.cfg") `
    -Value "res://addons/godot_osi/godot_osi.gdextension"

if (-not (Test-Path $Godot)) { throw "Godot binary not found: $Godot (pass -Godot <path>)" }

# Import once so `class_name` scripts (OsiPresetStore / OsiSettingsPanel /
# OsiSettingsConfig) populate global_script_class_cache.cfg. Unlike the
# native-only smokes, this test references project classes by name.
Write-Host "==> importing project (build global class cache)"
& $Godot --headless --import --path $proj
# --import may exit non-zero on benign warnings; the test run below is the gate.

Write-Host "==> running headless UI smoke test"
& $Godot --headless --path $proj --script "res://test/ui_preset_smoke.gd"
$code = $LASTEXITCODE
if ($code -eq 0) { Write-Host "UI OK" } else { Write-Host "UI FAILED ($code)" }
exit $code
