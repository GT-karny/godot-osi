# Build the extension, register it for a headless run, and execute the
# end-to-end integration smoke test (mock server -> receiver -> converter).
# Exits non-zero on failure so it can gate CI.
#
# Usage (from anywhere):
#   pwsh godot/test/run_integration.ps1 [-Godot <path-to-godot_console.exe>]
param(
    # Default points at the shared main checkout's temp/ (this project is a
    # worktree under worktrees/integration; the editor binary is not duplicated
    # per worktree). Pass -Godot to override.
    [string]$Godot = "$PSScriptRoot/../../../../temp/Godot_v4.6.3-stable_win64.exe/Godot_v4.6.3-stable_win64_console.exe"
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

# Register the GDExtension without needing a prior editor import: headless runs
# only load extensions listed here. (.godot/ is gitignored.)
New-Item -ItemType Directory -Force (Join-Path $proj ".godot") | Out-Null
Set-Content -Path (Join-Path $proj ".godot/extension_list.cfg") `
    -Value "res://addons/godot_osi/godot_osi.gdextension"

if (-not (Test-Path $Godot)) { throw "Godot binary not found: $Godot (pass -Godot <path>)" }

Write-Host "==> running headless integration smoke test"
& $Godot --headless --path $proj --script "res://test/integration_smoke.gd"
$code = $LASTEXITCODE
if ($code -eq 0) { Write-Host "INTEGRATION OK" } else { Write-Host "INTEGRATION FAILED ($code)" }
exit $code
