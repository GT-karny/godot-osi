//! Compile the ASAM OSI v3.7.0 protos and the gRPC service definitions into
//! Rust types + tonic client/server stubs.
//!
//! protoc is not required on the system: we use the `protoc-bin-vendored`
//! crate, which ships a protoc binary, and point tonic-build at it.

use std::path::PathBuf;

fn main() {
    // Provide a protoc binary without requiring a system install.
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("vendored protoc binary");
    std::env::set_var("PROTOC", protoc);

    // CARGO_MANIFEST_DIR = <repo>/crates/osi-types ; go up twice to the repo root.
    // Avoid `canonicalize()` on Windows: it returns a `\\?\` UNC path that protoc
    // cannot parse.
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let repo_root = manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("manifest dir has a grandparent")
        .to_path_buf();
    let proto_root = repo_root.join("proto");
    let osi3 = proto_root.join("osi3");
    let service = proto_root.join("service");

    // The service protos import "osi3/osi_groundtruth.proto" (resolved via
    // `proto_root`), and the OSI protos import sibling "osi_*.proto" files
    // (resolved via `osi3`). Both include dirs are required.
    let includes = [proto_root.clone(), osi3.clone()];

    let protos = [
        service.join("service_groundtruth.proto"),
        service.join("service_hostvehicledata.proto"),
    ];

    // tonic 0.14 moved prost-based codegen into `tonic-prost-build`.
    tonic_prost_build::configure()
        .build_client(true)
        // Server stubs are used by the bundled mock gRPC server.
        .build_server(true)
        .compile_protos(&protos, &includes)
        .expect("failed to compile OSI protos");

    println!("cargo:rerun-if-changed={}", proto_root.display());
}
