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

## esmini (RoadManager) — MPL-2.0
The binaries statically link esmini's RoadManager and CommonMini modules, used
to parse OpenDRIVE (`.xodr`) files and answer road-geometry queries for the
`OsiRoadNetwork` / `OsiRoadNetworkVisualizer` classes.

- Source: https://github.com/GT-karny/esmini

## pugixml — MIT
XML parser pulled in by esmini's RoadManager to read `.xodr` files.
Copyright (C) 2006-2018 Arseny Kapoulkine.

- Source: https://github.com/zeux/pugixml

## fmt — MIT
Formatting library pulled in by esmini's CommonMini.
Copyright (c) 2012-present Victor Zverovich and {fmt} contributors.

- Source: https://github.com/fmtlib/fmt

---

No MPL-covered source file has been modified in this distribution. The MPL
requires only that the corresponding Source Code Form remain available under
the MPL; the `.proto` files under `third_party/osi3/` (ASAM OSI) and the public
esmini source repository linked above satisfy that requirement.
