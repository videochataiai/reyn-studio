//! Process-level CAD bridge IPC tests (slice 1 — stub, no OCCT).

use reyn_studio::cad_bridge::{
    decode_f32_le_b64, decode_u32_le_b64, default_bridge_bin, stub_fixture_mesh, write_frame,
    CadBridgeClient, FrameError, MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES, PROTOCOL_SCHEMA,
    STUB_BRIDGE_VERSION, STUB_OCCT_VERSION, STUB_SLOW_MARKER,
};
use serde_json::Value;
use std::io::Write;
use std::time::Duration;

fn bridge_bin() -> std::path::PathBuf {
    default_bridge_bin().expect("CARGO_BIN_EXE_reyn-cad-bridge must be set (run via cargo test)")
}

#[test]
fn ipc_hello_and_fixture_mesh() {
    let mut client = CadBridgeClient::spawn(bridge_bin()).expect("spawn bridge");
    let hello = client.hello("ipc-hello").expect("hello");
    assert_eq!(hello.bridge_version, STUB_BRIDGE_VERSION);
    assert_eq!(hello.occt_version, STUB_OCCT_VERSION);

    let mesh = client
        .tessellate_step(
            "ipc-mesh",
            "/tmp/part.step",
            0.001,
            10_000,
            16,
            None,
            Duration::from_secs(5),
        )
        .expect("tessellate exchange")
        .expect("mesh ok");
    assert_eq!(mesh.triangle_count, 1);
    assert_eq!(mesh.length_unit, "metre");
    assert_eq!(mesh.positions, stub_fixture_mesh().0);
    assert_eq!(mesh.indices, stub_fixture_mesh().1);
    assert!(mesh.warnings.iter().any(|w| w.contains("stub bridge")));
    client.shutdown().expect("shutdown");
}

#[test]
fn ipc_assembly_without_occurrence_fails_closed() {
    let mut client = CadBridgeClient::spawn(bridge_bin()).expect("spawn bridge");
    let err = client
        .tessellate_step(
            "ipc-asm",
            "/tmp/widget_assembly.step",
            0.001,
            10_000,
            16,
            None,
            Duration::from_secs(5),
        )
        .expect("exchange")
        .expect_err("assembly must fail");
    assert_eq!(err.code, "occurrence_required");
    client.shutdown().expect("shutdown");
}

#[test]
fn ipc_cancel_interrupts_slow_tessellate() {
    let mut client = CadBridgeClient::spawn(bridge_bin()).expect("spawn bridge");
    client.hello("c-hello").expect("hello");

    let tessellate_id = "slow-ipc";
    client
        .write_raw(&serde_json::json!({
            "schema": PROTOCOL_SCHEMA,
            "op": "tessellate_step",
            "request_id": tessellate_id,
            "path": format!("/tmp/{STUB_SLOW_MARKER}_part.step"),
            "chord_tolerance": 0.001,
            "max_triangles": 1000,
            "max_shells": 8,
        }))
        .expect("write tessellate");
    std::thread::sleep(Duration::from_millis(120));
    client
        .write_raw(&serde_json::json!({
            "schema": PROTOCOL_SCHEMA,
            "op": "cancel",
            "request_id": "cancel-ipc",
            "target_request_id": tessellate_id,
        }))
        .expect("write cancel");

    let mut saw_cancelled = false;
    let mut saw_ack = false;
    for _ in 0..4 {
        let frame = client.read_raw(Duration::from_secs(3)).expect("frame");
        match frame.get("op").and_then(Value::as_str) {
            Some("cancel") if frame.get("ok").and_then(Value::as_bool) == Some(true) => {
                saw_ack = true;
            }
            Some("tessellate_step")
                if frame.get("code").and_then(Value::as_str) == Some("cancelled") =>
            {
                saw_cancelled = true;
            }
            _ => panic!("unexpected frame: {frame}"),
        }
        if saw_cancelled && saw_ack {
            break;
        }
    }
    assert!(saw_cancelled, "expected cancelled tessellate response");
    assert!(saw_ack, "expected cancel ack");
    let _ = client.shutdown();
}

#[test]
fn ipc_timeout_kills_slow_bridge() {
    let mut client = CadBridgeClient::spawn(bridge_bin()).expect("spawn bridge");
    client.hello("t-hello").expect("hello");
    let err = client
        .tessellate_step(
            "timeout-1",
            &format!("/tmp/{STUB_SLOW_MARKER}_timeout.step"),
            0.001,
            1000,
            8,
            None,
            Duration::from_millis(200),
        )
        .expect_err("must time out");
    assert!(matches!(err, FrameError::Timeout), "got {err}");
}

#[test]
fn ipc_oversize_request_fails_closed() {
    let mut child = std::process::Command::new(bridge_bin())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn");
    let mut stdin = child.stdin.take().expect("stdin");
    let mut stdout = child.stdout.take().expect("stdout");

    let too_big = (MAX_REQUEST_BYTES as u32).saturating_add(1);
    stdin.write_all(&too_big.to_le_bytes()).expect("len");
    stdin.write_all(&[0u8; 16]).expect("pad");
    let _ = stdin.flush();

    let response = reyn_studio::cad_bridge::read_frame(&mut stdout, MAX_RESPONSE_BYTES);
    match response {
        Ok(value) => {
            assert_eq!(value["ok"], false);
            assert_eq!(value["code"], "memory_limit");
        }
        Err(FrameError::Oversize { .. }) | Err(FrameError::Io(_)) => {}
        Err(other) => panic!("unexpected error: {other}"),
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn ipc_mesh_bytes_roundtrip_helpers() {
    let (positions, indices) = stub_fixture_mesh();
    let encoded_pos = {
        use base64::Engine as _;
        let mut raw = Vec::new();
        for v in &positions {
            raw.extend_from_slice(&v.to_le_bytes());
        }
        base64::engine::general_purpose::STANDARD.encode(raw)
    };
    let encoded_idx = {
        use base64::Engine as _;
        let mut raw = Vec::new();
        for v in &indices {
            raw.extend_from_slice(&v.to_le_bytes());
        }
        base64::engine::general_purpose::STANDARD.encode(raw)
    };
    assert_eq!(decode_f32_le_b64(&encoded_pos).unwrap(), positions);
    assert_eq!(decode_u32_le_b64(&encoded_idx).unwrap(), indices);
    let mut frame = Vec::new();
    write_frame(
        &mut frame,
        &serde_json::json!({"schema": PROTOCOL_SCHEMA, "op": "hello", "request_id": "x"}),
    )
    .unwrap();
    assert!(frame.len() > 4);
}
