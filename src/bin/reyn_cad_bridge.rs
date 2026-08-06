//! `reyn-cad-bridge` — out-of-process CAD translation stub (slice 1).
//!
//! Speaks `docs/occt_bridge_protocol.v1.json` over length-prefixed JSON stdio.
//! This binary does **not** link OpenCASCADE; it returns a fixed fixture mesh
//! so Studio can prove IPC, cancel, timeout, and oversize fail-closed behavior
//! before any LGPL bridge lands.

use reyn_studio::cad_bridge::{run_stub_stdio, STUB_BRIDGE_VERSION, STUB_OCCT_VERSION};

fn main() {
    if std::env::args().any(|arg| arg == "--version" || arg == "-V") {
        println!("reyn-cad-bridge {STUB_BRIDGE_VERSION} (occt={STUB_OCCT_VERSION})");
        return;
    }

    if let Err(error) = run_stub_stdio() {
        eprintln!("reyn-cad-bridge: {error}");
        std::process::exit(1);
    }
}
