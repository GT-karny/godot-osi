# Third-Party Notices

This addon ("godot-osi") is licensed under **MIT OR Apache-2.0**.
See `LICENSE-MIT` and `LICENSE-APACHE`.

The compiled binaries in `bin/` statically link and/or incorporate the
following third-party components.

## ASAM OSI (Open Simulation Interface) — MPL-2.0
Copyright (C) BMW AG and contributors.

The binaries contain data types generated from the OSI Protocol Buffer
definitions. Those `.proto` definitions are licensed under the Mozilla Public
License 2.0 and are included with this distribution under `third_party/osi3/`.

- Source: https://github.com/OpenSimulationInterface/open-simulation-interface
- License text: `third_party/osi3/LICENSE`

## godot-rust (gdext) — MPL-2.0
The binaries are built with the gdext bindings.

- Source: https://github.com/godot-rust/gdext

## tonic — MIT
- Source: https://github.com/hyperium/tonic

## prost — Apache-2.0
- Source: https://github.com/tokio-rs/prost

---

No MPL-covered source file has been modified in this distribution. The MPL
requires only that the corresponding Source Code Form remain available under
the MPL; the `.proto` files under `third_party/osi3/` satisfy that requirement.
