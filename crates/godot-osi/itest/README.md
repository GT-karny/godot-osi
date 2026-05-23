# Converter structural itest

Validates the build-time generated `convert_*` output **inside a running
Godot**, which plain `cargo test` can't do: constructing `Gd<Resource>` needs
the engine loaded. (The coordinate conversion in `converter::coords` *is*
engine-free and is covered by `cargo test -p godot-osi`; this only covers the
structural mirror.)

## What it checks

`OsiTestRig.make_sample_ground_truth()` (compiled only with the `itest` feature)
builds a fixed `osi3::GroundTruth` and runs it through `convert_ground_truth`.
[`test.gd`](test.gd) then asserts the typed snapshot preserves the **raw** OSI
values 1:1 across nested Resources, `Array<Gd<…>>`, and `Option<Gd<…>>` — the
agreed "A" geometry policy (no coordinate transform baked into the mirror).

## Run

```powershell
pwsh crates/godot-osi/itest/run_itest.ps1 [-Godot <path-to-godot_console.exe>]
```

The script: `cargo build -p godot-osi --features itest` → copies the dll into
`bin/` → writes `.godot/extension_list.cfg` (so the headless run loads the
extension without an editor import) → runs Godot headless with `test.gd` as the
SceneTree main loop. Exit code 0 = pass.

`bin/` and `.godot/` are generated and git-ignored.
