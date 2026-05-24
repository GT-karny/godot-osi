//! Build the esmini RoadManager C API (esminiRMLib) and statically link it into
//! this crate (and transitively into `godot_osi.dll`).
//!
//! Strategy (validated by PoC, see memory `opendrive-esmini-rmlib-poc.md`):
//!   1. Drive CMake to build only the `RoadManager` target with OSG/OSI/SUMO off
//!      and downloads disabled. This produces four static libs:
//!      RoadManager, CommonMini, pugixml_lib, fmt (no OSG/OSI/Python pulled in).
//!   2. Compile the thin `esminiRMLib.cpp` C wrapper ourselves with `cc` (the
//!      upstream CMake only defines it as a SHARED lib; we want it static), and
//!      link everything together.
//!
//! We never modify anything under `external/esmini` (its CLAUDE.md R1: the
//! EnvironmentSimulator core must stay pristine) — we only consume it.
//!
//! CRT note: Rust on MSVC always links the dynamic release CRT (`/MD`). We force
//! the esmini build to `Release` regardless of the cargo profile so both halves
//! agree on `/MD` and don't trip the linker.

use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let esmini = manifest.join("..").join("..").join("external").join("esmini");
    let env_sim = esmini.join("EnvironmentSimulator");

    if !esmini.join("CMakeLists.txt").exists() {
        panic!(
            "esmini submodule not found at {}.\n\
             Run: git submodule update --init external/esmini",
            esmini.display()
        );
    }

    ensure_fmt_submodule(&esmini);

    // 1. CMake build of the RoadManager target (pulls in CommonMini/pugixml/fmt).
    let dst = cmake::Config::new(&esmini)
        .define("USE_OSG", "OFF")
        .define("USE_OSI", "OFF")
        .define("USE_SUMO", "OFF")
        .define("USE_GTEST", "OFF")
        .define("DOWNLOAD_EXTERNALS", "OFF")
        .define("BUILD_EXAMPLES", "OFF")
        .build_target("RoadManager")
        // Always Release so the esmini libs use /MD like Rust's CRT.
        .profile("Release")
        .build();

    // With `build_target`, the cmake crate configures+builds under <dst>/build
    // and does not install. Locate each static lib in the (multi-config) tree.
    let build_dir = dst.join("build");
    for lib in ["RoadManager", "CommonMini", "pugixml_lib", "fmt"] {
        let dir = find_lib_dir(&build_dir, lib).unwrap_or_else(|| {
            panic!(
                "static lib '{lib}.lib' not found under {}",
                build_dir.display()
            )
        });
        println!("cargo:rustc-link-search=native={}", dir.display());
    }
    println!("cargo:rustc-link-lib=static=RoadManager");
    println!("cargo:rustc-link-lib=static=CommonMini");
    println!("cargo:rustc-link-lib=static=pugixml_lib");
    println!("cargo:rustc-link-lib=static=fmt");

    // CommonMini's UDP/logging needs Windows socket + multimedia timer libs.
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=dylib=ws2_32");
        println!("cargo:rustc-link-lib=dylib=winmm");
    }

    // 2. Compile the esminiRMLib.cpp C wrapper as a static lib and link it.
    let wrapper = env_sim.join("Libraries/esminiRMLib/esminiRMLib.cpp");
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .define("NOMINMAX", None)
        // fmt (pulled in via logger.hpp) static_asserts that the source is
        // compiled as UTF-8; CommonMini.hpp already sets _USE_MATH_DEFINES.
        .flag_if_supported("/utf-8")
        .include(env_sim.join("Libraries/esminiRMLib"))
        .include(env_sim.join("Modules/RoadManager"))
        .include(env_sim.join("Modules/CommonMini"))
        .include(env_sim.join("Modules/ScenarioEngine/SourceFiles"))
        .include(env_sim.join("Modules/ScenarioEngine/OSCTypeDefs"))
        .include(esmini.join("externals/pugixml"))
        .include(esmini.join("externals/fmt/include"))
        .file(&wrapper)
        // Our own shims exposing RoadManager structure the esminiRMLib C API
        // doesn't (road marks, geometry/OSI points, lanes, objects, topology,
        // signals, profiles, routes). See cpp/gt_common.hpp for conventions.
        .file(manifest.join("cpp/gt_roadmark.cpp"))
        .file(manifest.join("cpp/gt_geometry.cpp"))
        .file(manifest.join("cpp/gt_lane.cpp"))
        .file(manifest.join("cpp/gt_object.cpp"))
        .file(manifest.join("cpp/gt_topology.cpp"))
        .file(manifest.join("cpp/gt_signal.cpp"))
        .file(manifest.join("cpp/gt_misc.cpp"))
        .file(manifest.join("cpp/gt_route.cpp"))
        .compile("esminiRMLib_wrapper");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/ffi.rs");
    println!("cargo:rerun-if-changed=cpp/gt_common.hpp");
    println!("cargo:rerun-if-changed=cpp/gt_roadmark.cpp");
    println!("cargo:rerun-if-changed=cpp/gt_geometry.cpp");
    println!("cargo:rerun-if-changed=cpp/gt_lane.cpp");
    println!("cargo:rerun-if-changed=cpp/gt_object.cpp");
    println!("cargo:rerun-if-changed=cpp/gt_topology.cpp");
    println!("cargo:rerun-if-changed=cpp/gt_signal.cpp");
    println!("cargo:rerun-if-changed=cpp/gt_misc.cpp");
    println!("cargo:rerun-if-changed=cpp/gt_route.cpp");
    println!("cargo:rerun-if-changed={}", wrapper.display());
}

/// RoadManager links `externals/fmt`, which is a submodule inside esmini. A plain
/// `git submodule update --init external/esmini` does not fetch it, so make sure
/// it is present (init it if missing) before invoking CMake.
fn ensure_fmt_submodule(esmini: &Path) {
    let fmt_inc = esmini.join("externals/fmt/include/fmt/format.h");
    if fmt_inc.exists() {
        return;
    }
    let _ = Command::new("git")
        .arg("-C")
        .arg(esmini)
        .args(["submodule", "update", "--init", "--depth", "1", "externals/fmt"])
        .status();
    if !fmt_inc.exists() {
        panic!(
            "esmini's fmt submodule is missing and could not be initialized.\n\
             Run: git -C {} submodule update --init externals/fmt",
            esmini.display()
        );
    }
}

/// Recursively search `root` for a directory directly containing `<name>.lib`.
fn find_lib_dir(root: &Path, name: &str) -> Option<PathBuf> {
    let target = format!("{name}.lib");
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().and_then(|f| f.to_str()) == Some(target.as_str()) {
                return Some(dir);
            }
        }
    }
    None
}
