//! Python engine client. Spawns `reyn_engine.py` under the research venv, reads
//! `READY {port}`, connects over loopback TCP, and exchanges length-prefixed
//! frames. Runs on a worker thread; the UI talks to it via channels so inference
//! (seconds) never blocks the egui frame.
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

pub struct Field {
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
    pub scenario: String,
}

pub enum Cmd {
    ListModels,
    Predict { model: String, seed: u64 },
}

pub enum Msg {
    Status(String),
    Models(Vec<String>),
    Field(Field),
    Error(String),
}

pub struct EngineHandle {
    pub tx: Sender<Cmd>,
    pub rx: Receiver<Msg>,
}

pub fn research_dir() -> String {
    std::env::var("REYN_RESEARCH_DIR")
        .unwrap_or_else(|_| "/Users/hamza/Documents/Pioneer RI/reyn-research".to_string())
}

impl EngineHandle {
    /// Spawn the engine + worker thread. Returns immediately; readiness/errors
    /// arrive as `Msg::Status` / `Msg::Error`.
    pub fn spawn() -> EngineHandle {
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<Cmd>();
        let (msg_tx, msg_rx) = std::sync::mpsc::channel::<Msg>();
        thread::spawn(move || worker(cmd_rx, msg_tx));
        EngineHandle { tx: cmd_tx, rx: msg_rx }
    }
}

fn worker(cmd_rx: Receiver<Cmd>, msg_tx: Sender<Msg>) {
    let mut conn = match start() {
        Ok((stream, _child)) => {
            let _ = msg_tx.send(Msg::Status("engine ready".into()));
            // keep the child alive for the life of the thread
            std::mem::forget(_child);
            stream
        }
        Err(e) => { let _ = msg_tx.send(Msg::Error(format!("engine unavailable: {e}"))); return; }
    };
    while let Ok(cmd) = cmd_rx.recv() {
        let res = match cmd {
            Cmd::ListModels => request(&mut conn, r#"{"op":"list_models"}"#.into())
                .map(|(j, _)| Msg::Models(j["models"].as_array().map(|a|
                    a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default())),
            Cmd::Predict { model, seed } => {
                let req = format!(r#"{{"op":"predict_field","model":"{model}","seed":{seed}}}"#);
                request(&mut conn, req).map(|(j, payload)| {
                    if !j["ok"].as_bool().unwrap_or(false) {
                        return Msg::Error(j["error"].as_str().unwrap_or("predict failed").into());
                    }
                    let shape: Vec<usize> = j["shape"].as_array().unwrap_or(&vec![])
                        .iter().filter_map(|v| v.as_u64().map(|n| n as usize)).collect();
                    let data: Vec<f32> = payload.chunks_exact(4)
                        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect();
                    Msg::Field(Field { shape, data, scenario: j["scenario"].as_str().unwrap_or("").into() })
                })
            }
        };
        let _ = msg_tx.send(res.unwrap_or_else(|e| Msg::Error(format!("engine io: {e}"))));
    }
}

fn start() -> std::io::Result<(TcpStream, Child)> {
    let research = research_dir();
    let python = format!("{research}/.venv/bin/python");
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/engine/reyn_engine.py");
    let mut child = Command::new(&python)
        .args(["-u", script, "--research-dir", &research])
        .stdout(Stdio::piped()).stderr(Stdio::null())
        .spawn()?;
    let stdout = child.stdout.take().expect("piped stdout");
    let mut line = String::new();
    BufReader::new(stdout).read_line(&mut line)?;
    let json = line.trim().strip_prefix("READY ")
        .ok_or_else(|| std::io::Error::other(format!("bad engine startup: {line}")))?;
    let port = serde_json::from_str::<serde_json::Value>(json)
        .ok().and_then(|v| v["port"].as_u64())
        .ok_or_else(|| std::io::Error::other("no port in READY"))?;
    let stream = TcpStream::connect(("127.0.0.1", port as u16))?;
    Ok((stream, child))
}

fn request(conn: &mut TcpStream, json: String) -> std::io::Result<(serde_json::Value, Vec<u8>)> {
    let jb = json.as_bytes();
    let body_len = 4 + jb.len();
    conn.write_all(&(body_len as u32).to_le_bytes())?;
    conn.write_all(&(jb.len() as u32).to_le_bytes())?;
    conn.write_all(jb)?;
    // read response
    let mut lenb = [0u8; 4];
    conn.read_exact(&mut lenb)?;
    let total = u32::from_le_bytes(lenb) as usize;
    let mut body = vec![0u8; total];
    conn.read_exact(&mut body)?;
    let jl = u32::from_le_bytes([body[0], body[1], body[2], body[3]]) as usize;
    let value: serde_json::Value = serde_json::from_slice(&body[4..4 + jl])
        .map_err(|e| std::io::Error::other(format!("json: {e}")))?;
    Ok((value, body[4 + jl..].to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn wait_for(h: &EngineHandle, pred: impl Fn(&Msg) -> bool, secs: u64) -> Option<Msg> {
        let deadline = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < deadline {
            if let Ok(m) = h.rx.recv_timeout(Duration::from_millis(200)) {
                if pred(&m) { return Some(m); }
            }
        }
        None
    }

    /// End-to-end bridge test: spawns the real Python engine (needs the research
    /// venv + checkpoints) and verifies list_models + a predicted 3D field.
    #[test]
    fn engine_round_trip() {
        let h = EngineHandle::spawn();
        h.tx.send(Cmd::ListModels).unwrap();
        assert!(matches!(wait_for(&h, |m| matches!(m, Msg::Models(_)), 20),
            Some(Msg::Models(ref v)) if !v.is_empty()), "no models listed");

        h.tx.send(Cmd::Predict { model: "flow3d_obs_v1.pth".into(), seed: 3 }).unwrap();
        match wait_for(&h, |m| matches!(m, Msg::Field(_) | Msg::Error(_)), 40) {
            Some(Msg::Field(f)) => {
                assert_eq!(f.shape, vec![3, 32, 32, 32]);
                assert_eq!(f.data.len(), 3 * 32 * 32 * 32);
                assert!(!crate::flow::from_field(&f.shape, &f.data).is_empty(),
                    "field produced no particles");
            }
            Some(Msg::Error(e)) => panic!("engine error: {e}"),
            _ => panic!("timed out waiting for the model field"),
        }
    }
}
