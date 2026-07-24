//! Python engine client. Spawns `reyn_engine.py` under the research venv, reads
//! `READY {port}`, connects over loopback TCP, and exchanges length-prefixed
//! frames. Runs on a worker thread; the UI talks to it via channels so inference
//! (seconds) never blocks the egui frame.
use crate::benchmark_evidence::{
    InspectorMaps, InspectorVariable, INSPECTOR_DERIVATIVE, INSPECTOR_DOMAIN, INSPECTOR_LAYOUT,
    INSPECTOR_PRESSURE, INSPECTOR_PROTOCOL_VERSION, INSPECTOR_SCHEMA,
};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineConfig {
    pub research_dir: String,
    pub python_path: String,
    pub device: String,
}

impl Default for EngineConfig {
    fn default() -> Self {
        let research_dir = research_dir();
        let local_python = format!("{research_dir}/.venv/bin/python");
        let python_path = std::env::var("REYN_PYTHON").unwrap_or_else(|_| {
            if Path::new(&local_python).is_file() {
                local_python
            } else {
                "python3".into()
            }
        });
        Self {
            research_dir,
            python_path,
            device: std::env::var("REYN_DEVICE").unwrap_or_else(|_| "auto".into()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ModelCard {
    pub id: String,
    pub name: String,
    pub managed: bool,
    pub size_bytes: u64,
    pub modified_unix: u64,
    pub checkpoint_sha256: String,
    pub status: String,
    pub status_detail: String,
    pub dimension: u32,
    pub grid: u32,
    pub in_channels: u32,
    pub out_channels: u32,
    pub max_steps: u32,
    pub epoch: u32,
    pub declared_epochs: u32,
    pub checkpoint_role: String,
    pub scenario: String,
    pub source_digest: Option<String>,
    pub physics_contract: String,
    pub support: Vec<String>,
    pub limitations: Vec<String>,
    pub benchmark_report_hashes: Vec<String>,
    pub unknown_fields: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelValidationIssue {
    pub code: String,
    pub field: String,
    pub message: String,
    pub severity: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelValidation {
    pub accepted: bool,
    pub status: String,
    pub summary: String,
    pub issues: Vec<ModelValidationIssue>,
    pub candidate_name: Option<String>,
    pub candidate_sha256: Option<String>,
}

pub struct Field {
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
    pub scenario: String,
}

/// A 2D field for the pressure-recovery view: model velocity plus recovered
/// pressure (`ai` = `[3,N,N]` u,v,p), optional solver reference in the legacy
/// protocol field `truth`, and separate self-consistency/reference metrics.
pub struct Field2D {
    pub n: usize,
    pub ai: Vec<f32>,
    pub truth: Option<Vec<f32>>,
    pub horizon: u32,
    pub dt_frame: f32,
    pub peak_p: f32,
    pub low_p: f32,
    pub semigroup: Option<f32>,
    pub rel_l2: Option<f32>,
    pub persist: Option<f32>,
    pub p_residual: f32,
    pub p_iters: u32,
    pub p_method: String,
    pub scenario: String,
}

pub enum Cmd {
    ListModels,
    ImportModel {
        path: String,
    },
    DeleteModel {
        model: String,
    },
    Predict {
        model: String,
        seed: u64,
    },
    Predict2D {
        model: String,
        steps: u32,
        seed: u64,
        want_truth: bool,
        method: String,   // "spectral" | "fd"
        tolerance: f32,   // FD stop tolerance
        boundary: String, // "periodic" | "dirichlet"
    },
    /// Flow Painter: advance a user-painted `[2,N,N]` velocity IC. Stateless —
    /// the IC rides with every request (a 128² IC is ~130 KB over loopback).
    PredictIC {
        model: String,
        steps: u32,
        ic: std::sync::Arc<Vec<f32>>,
    },
    /// CAD flow analysis: a voxelized `[N³]` STL mask becomes a wind-tunnel
    /// case (engine caches the solver warmup per mask, so horizon changes are
    /// one model pass).
    CadPredict {
        request_id: String,
        model: String,
        steps: u32,
        mask: std::sync::Arc<Vec<f32>>,
        reynolds: f32,
        characteristic_length_solver: f32,
        reference_length_m: f32,
        velocity_mps: f32,
        density_kg_m3: f32,
        reference_pressure_pa: f32,
    },
    /// N5 — run the benchmark suite (seeds × horizons) on a 2D checkpoint.
    RunBenchmark {
        model: String,
        seeds: Vec<u32>,
        horizons: Vec<u32>,
    },
    /// N5.2 — fetch field/error/spectral evidence only for the selected cell.
    InspectBenchmarkCell {
        model: String,
        seed: u32,
        horizon: u32,
    },
}

/// CAD prediction: model velocity + recovered pressure + the smoothed mask.
pub struct CadField {
    pub request_id: String,
    pub n: usize,
    pub vel: Vec<f32>,      // [3,N,N,N]
    pub pressure: Vec<f32>, // physical Pa, [N,N,N]
    pub mask: Vec<f32>,     // [N,N,N]
    pub cp: Vec<f32>,       // physical pressure coefficient, [N,N,N]
    pub traction: Vec<f32>, // [3,N,N,N], physical Pa on diffuse surface
    pub horizon: u32,
    pub reynolds: f32,
    pub characteristic_length_solver: f32,
    pub solver_dt: f32,
    pub solver_stride: u32,
    pub warmup_steps: u32,
    pub dt_frame: f32,
    pub force_coefficients: [f32; 3],
    pub moment_coefficients: [f32; 3],
    pub force_newtons: [f32; 3],
    pub moment_newton_meters: [f32; 3],
    pub surface_area_m2: f32,
    pub pressure_force_fraction: f32,
    pub load_hotspot: [f32; 3],
    pub suction_hotspot: [f32; 3],
    pub divergence_rms: f32,
    pub wake_deficit_peak: f32,
    pub wake_deficit_mean: f32,
    pub load_method: String,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchSeedProvenance {
    pub seed: u32,
    pub stream: String,
    pub overlap: bool,
}

/// Metadata-backed leak/provenance findings. `verdict` is `clean`, `flagged`,
/// or `unknown`; missing legacy metadata never silently becomes clean.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchProvenance {
    pub verdict: String,
    pub training_seed: Option<i64>,
    pub mixed_fork_seed: Option<i64>,
    pub mixed_fork_used: bool,
    pub validation_seed: Option<i64>,
    pub dataset: String,
    pub benchmark_seeds: Vec<BenchSeedProvenance>,
    pub overlap_count: u32,
    pub overlap_pct: f32,
    pub epoch: Option<u32>,
    pub declared_epochs: Option<u32>,
    pub checkpoint_role: String,
    pub final_epoch_status: String,
    pub selection_metric: String,
    pub selection_stream: String,
    pub source_fingerprint_present: bool,
    pub source_fingerprint_digest: Option<String>,
    pub legacy_unknown: Vec<String>,
    pub flags: Vec<String>,
}

/// N5 suite result: RelL2 + persistence per exact (seed, horizon), plus the
/// checkpoint-derived provenance verdict for those seeds.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchResult {
    pub model: String,
    pub seeds: Vec<u32>,
    pub horizons: Vec<u32>,
    pub rel: Vec<Vec<f32>>,
    pub persist: Vec<Vec<f32>>,
    pub global_rel: f32,
    pub runtime_s: f32,
    pub grid: u32,
    pub epoch: u32,
    pub dt_frame: f32,
    pub provenance: BenchProvenance,
}

/// On-demand evidence for one selected benchmark cell. `maps` carries
/// velocity/vorticity/recovered-pressure/divergence model, solver-reference,
/// and spatial-error maps.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BenchInspector {
    pub seed: u32,
    pub horizon: u32,
    pub n: usize,
    pub maps: InspectorMaps,
    pub seed_stream: String,
    pub provenance_status: String,
    pub rel_l2: f32,
    pub persist_rel_l2: f32,
    pub improvement_ratio: f32,
    pub mean_abs_error: f32,
    pub p95_abs_error: f32,
    pub max_abs_error: f32,
    pub divergence_model_rms: f32,
    pub divergence_truth_rms: f32,
    pub divergence_error_rms: f32,
    pub spectrum_rel_l2: f32,
    pub spectrum_k: Vec<f32>,
    pub spectrum_model: Vec<f32>,
    pub spectrum_truth: Vec<f32>,
}

pub enum Msg {
    Status(String),
    Models(Vec<ModelCard>),
    ModelImported {
        model: ModelCard,
        models: Vec<ModelCard>,
    },
    ModelImportRejected(ModelValidation),
    ModelDeleted {
        model: String,
        models: Vec<ModelCard>,
    },
    Field(Field),
    Field2D(Field2D),
    CadField(CadField),
    Benchmark(BenchResult),
    BenchmarkInspector(BenchInspector),
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
    #[allow(dead_code)] // retained for engine integration tests and embedders
    pub fn spawn() -> EngineHandle {
        Self::spawn_with_config(EngineConfig::default())
    }

    pub fn spawn_with_config(config: EngineConfig) -> EngineHandle {
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<Cmd>();
        let (msg_tx, msg_rx) = std::sync::mpsc::channel::<Msg>();
        thread::spawn(move || worker(cmd_rx, msg_tx, config));
        EngineHandle {
            tx: cmd_tx,
            rx: msg_rx,
        }
    }
}

/// Human-readable compute device label for status chips ("mps" → "Apple GPU (MPS)").
fn device_label(device: &str) -> String {
    match device {
        "mps" => "Apple GPU (MPS)".into(),
        "cuda" => "NVIDIA GPU (CUDA)".into(),
        "cpu" => "CPU".into(),
        other => other.into(),
    }
}

fn worker(cmd_rx: Receiver<Cmd>, msg_tx: Sender<Msg>, config: EngineConfig) {
    let mut conn = match start(&config) {
        Ok((stream, _child, device)) => {
            let _ = msg_tx.send(Msg::Status(format!(
                "● Engine ready · {}",
                device_label(&device)
            )));
            // keep the child alive for the life of the thread
            std::mem::forget(_child);
            stream
        }
        Err(e) => {
            let _ = msg_tx.send(Msg::Error(format!("engine unavailable: {e}")));
            return;
        }
    };
    while let Ok(cmd) = cmd_rx.recv() {
        let res = match cmd {
            Cmd::ListModels => request(&mut conn, r#"{"op":"list_models"}"#.into(), &[])
                .map(|(j, _)| Msg::Models(parse_model_cards(&j["models"]))),
            Cmd::ImportModel { path } => {
                let req = serde_json::json!({"op": "import_model", "path": path}).to_string();
                request(&mut conn, req, &[]).map(|(j, _)| {
                    if !j["ok"].as_bool().unwrap_or(false) {
                        if let Some(validation) = parse_model_validation(&j["validation"]) {
                            return Msg::ModelImportRejected(validation);
                        }
                        return Msg::Error(
                            j["error"].as_str().unwrap_or("model import failed").into(),
                        );
                    }
                    let Some(model) = parse_model_card(&j["imported"]) else {
                        return Msg::Error("model import returned malformed metadata".into());
                    };
                    Msg::ModelImported {
                        model,
                        models: parse_model_cards(&j["models"]),
                    }
                })
            }
            Cmd::DeleteModel { model } => {
                let req = serde_json::json!({"op": "delete_model", "model": model}).to_string();
                request(&mut conn, req, &[]).map(|(j, _)| {
                    if !j["ok"].as_bool().unwrap_or(false) {
                        return Msg::Error(
                            j["error"].as_str().unwrap_or("model delete failed").into(),
                        );
                    }
                    Msg::ModelDeleted {
                        model: j["deleted"].as_str().unwrap_or("").into(),
                        models: parse_model_cards(&j["models"]),
                    }
                })
            }
            Cmd::Predict { model, seed } => {
                let req = serde_json::json!({
                    "op": "predict_field",
                    "model": model,
                    "seed": seed,
                })
                .to_string();
                request(&mut conn, req, &[]).map(|(j, payload)| {
                    if !j["ok"].as_bool().unwrap_or(false) {
                        return Msg::Error(j["error"].as_str().unwrap_or("predict failed").into());
                    }
                    let shape: Vec<usize> = j["shape"]
                        .as_array()
                        .unwrap_or(&vec![])
                        .iter()
                        .filter_map(|v| v.as_u64().map(|n| n as usize))
                        .collect();
                    let data: Vec<f32> = payload
                        .chunks_exact(4)
                        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                        .collect();
                    Msg::Field(Field {
                        shape,
                        data,
                        scenario: j["scenario"].as_str().unwrap_or("").into(),
                    })
                })
            }
            Cmd::Predict2D {
                model,
                steps,
                seed,
                want_truth,
                method,
                tolerance,
                boundary,
            } => {
                let req = serde_json::json!({
                    "op": "predict2d",
                    "model": model,
                    "steps": steps,
                    "seed": seed,
                    "want_truth": want_truth,
                    "method": method,
                    "tolerance": tolerance,
                    "boundary": boundary,
                    "max_iter": 600,
                })
                .to_string();
                request(&mut conn, req, &[]).map(|(j, payload)| parse_field2d(&j, &payload))
            }
            Cmd::PredictIC { model, steps, ic } => {
                let req = serde_json::json!({"op": "predict_ic", "model": model, "steps": steps})
                    .to_string();
                let bytes: Vec<u8> = ic.iter().flat_map(|v| v.to_le_bytes()).collect();
                request(&mut conn, req, &bytes).map(|(j, payload)| parse_field2d(&j, &payload))
            }
            Cmd::CadPredict {
                request_id,
                model,
                steps,
                mask,
                reynolds,
                characteristic_length_solver,
                reference_length_m,
                velocity_mps,
                density_kg_m3,
                reference_pressure_pa,
            } => {
                let req = serde_json::json!({
                    "op": "predict_cad",
                    "request_id": request_id,
                    "model": model,
                    "steps": steps,
                    "reynolds": reynolds,
                    "char_len": characteristic_length_solver,
                    "reference_length_m": reference_length_m,
                    "velocity_mps": velocity_mps,
                    "density_kg_m3": density_kg_m3,
                    "reference_pressure_pa": reference_pressure_pa,
                })
                .to_string();
                let bytes: Vec<u8> = mask.iter().flat_map(|v| v.to_le_bytes()).collect();
                request(&mut conn, req, &bytes).map(|(j, payload)| {
                    if !j["ok"].as_bool().unwrap_or(false) {
                        return Msg::Error(
                            j["error"].as_str().unwrap_or("predict_cad failed").into(),
                        );
                    }
                    let n = j["shape"][1].as_u64().unwrap_or(0) as usize;
                    let all: Vec<f32> = payload
                        .chunks_exact(4)
                        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                        .collect();
                    let cube = n * n * n;
                    if all.len() < 9 * cube {
                        return Msg::Error("short CAD payload".into());
                    }
                    let f = |k: &str| j[k].as_f64().unwrap_or(0.0) as f32;
                    let vector = |key: &str| -> [f32; 3] {
                        [
                            j[key][0].as_f64().unwrap_or(0.0) as f32,
                            j[key][1].as_f64().unwrap_or(0.0) as f32,
                            j[key][2].as_f64().unwrap_or(0.0) as f32,
                        ]
                    };
                    Msg::CadField(CadField {
                        request_id: j["request_id"].as_str().unwrap_or("").to_string(),
                        n,
                        vel: all[..3 * cube].to_vec(),
                        pressure: all[3 * cube..4 * cube].to_vec(),
                        mask: all[4 * cube..5 * cube].to_vec(),
                        cp: all[5 * cube..6 * cube].to_vec(),
                        traction: all[6 * cube..9 * cube].to_vec(),
                        horizon: j["horizon"].as_u64().unwrap_or(0) as u32,
                        reynolds: f("reynolds"),
                        characteristic_length_solver: f("char_len"),
                        solver_dt: f("solver_dt"),
                        solver_stride: j["solver_stride"].as_u64().unwrap_or(0) as u32,
                        warmup_steps: j["warmup_steps"].as_u64().unwrap_or(0) as u32,
                        dt_frame: f("dt_frame"),
                        force_coefficients: vector("force_coefficients"),
                        moment_coefficients: vector("moment_coefficients"),
                        force_newtons: vector("force_newtons"),
                        moment_newton_meters: vector("moment_newton_meters"),
                        surface_area_m2: f("surface_area_m2"),
                        pressure_force_fraction: f("pressure_force_fraction"),
                        load_hotspot: vector("load_hotspot"),
                        suction_hotspot: vector("suction_hotspot"),
                        divergence_rms: f("divergence_rms"),
                        wake_deficit_peak: f("wake_deficit_peak"),
                        wake_deficit_mean: f("wake_deficit_mean"),
                        load_method: j["load_method"].as_str().unwrap_or("unknown").to_string(),
                        warnings: j["warnings"]
                            .as_array()
                            .map(|warnings| {
                                warnings
                                    .iter()
                                    .filter_map(|warning| warning.as_str().map(str::to_owned))
                                    .collect()
                            })
                            .unwrap_or_default(),
                    })
                })
            }
            Cmd::RunBenchmark {
                model,
                seeds,
                horizons,
            } => {
                let req = serde_json::json!({
                    "op": "run_benchmark",
                    "model": model,
                    "seeds": seeds,
                    "horizons": horizons,
                })
                .to_string();
                request(&mut conn, req, &[]).map(|(j, _)| parse_benchmark(&j, model))
            }
            Cmd::InspectBenchmarkCell {
                model,
                seed,
                horizon,
            } => {
                let req = serde_json::json!({
                    "op": "inspect_benchmark_cell",
                    "model": model,
                    "seed": seed,
                    "horizon": horizon,
                    "evidence_schema": INSPECTOR_SCHEMA,
                })
                .to_string();
                request(&mut conn, req, &[])
                    .map(|(j, payload)| parse_benchmark_inspector(&j, &payload))
            }
        };
        let _ = msg_tx.send(res.unwrap_or_else(|e| Msg::Error(format!("engine io: {e}"))));
    }
}

fn parse_model_card(value: &serde_json::Value) -> Option<ModelCard> {
    Some(ModelCard {
        id: value["id"].as_str()?.into(),
        name: value["name"].as_str()?.into(),
        managed: value["managed"].as_bool().unwrap_or(false),
        size_bytes: value["size_bytes"].as_u64().unwrap_or(0),
        modified_unix: value["modified_unix"].as_u64().unwrap_or(0),
        checkpoint_sha256: value["checkpoint_sha256"].as_str().unwrap_or("").into(),
        status: value["status"].as_str().unwrap_or("invalid").into(),
        status_detail: value["status_detail"].as_str().unwrap_or("").into(),
        dimension: value["dimension"].as_u64().unwrap_or(0) as u32,
        grid: value["grid"].as_u64().unwrap_or(0) as u32,
        in_channels: value["in_channels"].as_u64().unwrap_or(0) as u32,
        out_channels: value["out_channels"].as_u64().unwrap_or(0) as u32,
        max_steps: value["max_steps"].as_u64().unwrap_or(0) as u32,
        epoch: value["epoch"].as_u64().unwrap_or(0) as u32,
        declared_epochs: value["declared_epochs"].as_u64().unwrap_or(0) as u32,
        checkpoint_role: value["checkpoint_role"]
            .as_str()
            .unwrap_or("unknown")
            .into(),
        scenario: value["scenario"].as_str().unwrap_or("unknown").into(),
        source_digest: value["source_digest"].as_str().map(str::to_owned),
        physics_contract: value["physics_contract"]
            .as_str()
            .unwrap_or("unknown")
            .into(),
        support: json_strings(&value["support"]),
        limitations: json_strings(&value["limitations"]),
        benchmark_report_hashes: json_strings(&value["benchmark_report_hashes"]),
        unknown_fields: json_strings(&value["unknown_fields"]),
    })
}

fn parse_model_validation(value: &serde_json::Value) -> Option<ModelValidation> {
    let issues = value["issues"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|issue| {
                    Some(ModelValidationIssue {
                        code: issue["code"].as_str()?.into(),
                        field: issue["field"].as_str().unwrap_or("checkpoint").into(),
                        message: issue["message"].as_str()?.into(),
                        severity: issue["severity"].as_str().unwrap_or("error").into(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(ModelValidation {
        accepted: value["accepted"].as_bool()?,
        status: value["status"].as_str()?.into(),
        summary: value["summary"].as_str()?.into(),
        issues,
        candidate_name: value["candidate"]["name"].as_str().map(str::to_owned),
        candidate_sha256: value["candidate"]["checkpoint_sha256"]
            .as_str()
            .map(str::to_owned),
    })
}

fn parse_model_cards(value: &serde_json::Value) -> Vec<ModelCard> {
    value
        .as_array()
        .map(|cards| cards.iter().filter_map(parse_model_card).collect())
        .unwrap_or_default()
}

/// Parse a `[3,N,N]` (+ optional truth) field response into `Msg::Field2D` —
/// shared by `predict2d` and `predict_ic` (identical response shape).
fn parse_field2d(j: &serde_json::Value, payload: &[u8]) -> Msg {
    if !j["ok"].as_bool().unwrap_or(false) {
        return Msg::Error(j["error"].as_str().unwrap_or("predict failed").into());
    }
    let n = j["shape"][1].as_u64().unwrap_or(0) as usize;
    let all: Vec<f32> = payload
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    let plane = n * n;
    let ai = all
        .get(..3 * plane)
        .map(<[f32]>::to_vec)
        .unwrap_or_default();
    let has_truth = j["has_truth"].as_bool().unwrap_or(false);
    let truth = if has_truth && all.len() >= 6 * plane {
        Some(all[3 * plane..6 * plane].to_vec())
    } else {
        None
    };
    let f = |k: &str| j[k].as_f64().map(|v| v as f32);
    Msg::Field2D(Field2D {
        n,
        ai,
        truth,
        horizon: j["horizon"].as_u64().unwrap_or(0) as u32,
        dt_frame: f("dt_frame").unwrap_or(0.04),
        peak_p: f("peak_p").unwrap_or(0.0),
        low_p: f("low_p").unwrap_or(0.0),
        semigroup: f("semigroup"),
        rel_l2: f("rel_l2"),
        persist: f("persist"),
        p_residual: f("p_residual").unwrap_or(0.0),
        p_iters: j["p_iters"].as_u64().unwrap_or(0) as u32,
        p_method: j["method"].as_str().unwrap_or("spectral").into(),
        scenario: j["scenario"].as_str().unwrap_or("").into(),
    })
}

fn json_u32s(value: &serde_json::Value) -> Vec<u32> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_u64().map(|number| number as u32))
                .collect()
        })
        .unwrap_or_default()
}

fn json_f32s(value: &serde_json::Value) -> Vec<f32> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_f64().map(|number| number as f32))
                .collect()
        })
        .unwrap_or_default()
}

fn json_strings(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

fn json_matrix(value: &serde_json::Value) -> Vec<Vec<f32>> {
    value
        .as_array()
        .map(|rows| rows.iter().map(json_f32s).collect())
        .unwrap_or_default()
}

fn parse_provenance(value: &serde_json::Value) -> BenchProvenance {
    let benchmark_seeds = value["benchmark_seeds"]
        .as_array()
        .map(|records| {
            records
                .iter()
                .map(|record| BenchSeedProvenance {
                    seed: record["seed"].as_u64().unwrap_or(0) as u32,
                    stream: record["stream"].as_str().unwrap_or("unknown").into(),
                    overlap: record["overlap"].as_bool().unwrap_or(false),
                })
                .collect()
        })
        .unwrap_or_default();
    BenchProvenance {
        verdict: value["verdict"].as_str().unwrap_or("unknown").into(),
        training_seed: value["training_seed"].as_i64(),
        mixed_fork_seed: value["mixed_fork_seed"].as_i64(),
        mixed_fork_used: value["mixed_fork_used"].as_bool().unwrap_or(false),
        validation_seed: value["validation_seed"].as_i64(),
        dataset: value["dataset"].as_str().unwrap_or("legacy/unknown").into(),
        benchmark_seeds,
        overlap_count: value["overlap_count"].as_u64().unwrap_or(0) as u32,
        overlap_pct: value["overlap_pct"].as_f64().unwrap_or(0.0) as f32,
        epoch: value["epoch"].as_u64().map(|number| number as u32),
        declared_epochs: value["declared_epochs"]
            .as_u64()
            .map(|number| number as u32),
        checkpoint_role: value["checkpoint_role"]
            .as_str()
            .unwrap_or("legacy/unknown")
            .into(),
        final_epoch_status: value["final_epoch_status"]
            .as_str()
            .unwrap_or("unknown")
            .into(),
        selection_metric: value["selection_metric"]
            .as_str()
            .unwrap_or("legacy/unknown")
            .into(),
        selection_stream: value["selection_stream"]
            .as_str()
            .unwrap_or("unknown")
            .into(),
        source_fingerprint_present: value["source_fingerprint_present"]
            .as_bool()
            .unwrap_or(false),
        source_fingerprint_digest: value["source_fingerprint_digest"]
            .as_str()
            .map(str::to_owned),
        legacy_unknown: json_strings(&value["legacy_unknown"]),
        flags: json_strings(&value["flags"]),
    }
}

fn parse_benchmark(j: &serde_json::Value, model: String) -> Msg {
    if !j["ok"].as_bool().unwrap_or(false) {
        return Msg::Error(j["error"].as_str().unwrap_or("benchmark failed").into());
    }
    let seeds = json_u32s(&j["seeds"]);
    let horizons = json_u32s(&j["horizons"]);
    let rel = json_matrix(&j["rel"]);
    let persist = json_matrix(&j["persist"]);
    let valid_shape = !seeds.is_empty()
        && !horizons.is_empty()
        && rel.len() == seeds.len()
        && persist.len() == seeds.len()
        && rel.iter().all(|row| row.len() == horizons.len())
        && persist.iter().all(|row| row.len() == horizons.len());
    if !valid_shape {
        return Msg::Error("malformed benchmark matrix".into());
    }
    Msg::Benchmark(BenchResult {
        model,
        seeds,
        horizons,
        rel,
        persist,
        global_rel: j["global_rel"].as_f64().unwrap_or(0.0) as f32,
        runtime_s: j["runtime_s"].as_f64().unwrap_or(0.0) as f32,
        grid: j["grid"].as_u64().unwrap_or(0) as u32,
        epoch: j["epoch"].as_u64().unwrap_or(0) as u32,
        dt_frame: j["dt_frame"].as_f64().unwrap_or(0.0) as f32,
        provenance: parse_provenance(&j["provenance"]),
    })
}

fn parse_benchmark_inspector(j: &serde_json::Value, payload: &[u8]) -> Msg {
    if !j["ok"].as_bool().unwrap_or(false) {
        return Msg::Error(
            j["error"]
                .as_str()
                .unwrap_or("benchmark inspector failed")
                .into(),
        );
    }
    let shape: Vec<usize> = j["shape"]
        .as_array()
        .map(|dims| {
            dims.iter()
                .filter_map(|dim| dim.as_u64().map(|value| value as usize))
                .collect()
        })
        .unwrap_or_default();
    if shape.len() != 4
        || shape[0] != InspectorVariable::ALL.len()
        || shape[1] != 3
        || shape[2] == 0
        || shape[2] != shape[3]
        || payload.len() % 4 != 0
        || j["protocol_version"].as_u64() != Some(INSPECTOR_PROTOCOL_VERSION)
        || j["layout"].as_str() != Some(INSPECTOR_LAYOUT)
        || j["domain"].as_str() != Some(INSPECTOR_DOMAIN)
        || j["derivative"].as_str() != Some(INSPECTOR_DERIVATIVE)
        || j["pressure"].as_str() != Some(INSPECTOR_PRESSURE)
    {
        return Msg::Error("malformed benchmark inspector payload".into());
    }
    let n = shape[2];
    let all: Vec<f32> = payload
        .chunks_exact(4)
        .map(|bytes| f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        .collect();
    let variables = json_strings(&j["variables"]);
    let units = json_strings(&j["units"]);
    let expected_units: Vec<_> = InspectorVariable::ALL
        .iter()
        .map(|variable| variable.unit_key().to_owned())
        .collect();
    let panel_sources: Vec<Vec<String>> = j["panel_sources"]
        .as_array()
        .map(|rows| rows.iter().map(json_strings).collect())
        .unwrap_or_default();
    let expected_panel_sources: Vec<Vec<String>> = InspectorVariable::ALL
        .iter()
        .map(|variable| {
            vec![
                variable.model_source().to_owned(),
                variable.reference_source().to_owned(),
                "DERIVED".to_owned(),
            ]
        })
        .collect();
    if units != expected_units || panel_sources != expected_panel_sources {
        return Msg::Error("malformed benchmark inspector source/unit metadata".into());
    }
    let signed: Vec<bool> = j["signed"]
        .as_array()
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_bool)
                .collect()
        })
        .unwrap_or_default();
    let maps = match InspectorMaps::from_protocol(
        j["schema"].as_str().unwrap_or(""),
        n,
        &variables,
        &signed,
        &all,
    ) {
        Ok(maps) => maps,
        Err(error) => return Msg::Error(format!("malformed benchmark inspector maps: {error}")),
    };
    let spectrum_k = json_f32s(&j["spectrum_k"]);
    let spectrum_model = json_f32s(&j["spectrum_model"]);
    let spectrum_truth = json_f32s(&j["spectrum_truth"]);
    if spectrum_k.is_empty()
        || spectrum_model.len() != spectrum_k.len()
        || spectrum_truth.len() != spectrum_k.len()
    {
        return Msg::Error("malformed benchmark spectrum".into());
    }
    let f = |key: &str| j[key].as_f64().unwrap_or(0.0) as f32;
    Msg::BenchmarkInspector(BenchInspector {
        seed: j["seed"].as_u64().unwrap_or(0) as u32,
        horizon: j["horizon"].as_u64().unwrap_or(0) as u32,
        n,
        maps,
        seed_stream: j["seed_stream"].as_str().unwrap_or("unknown").into(),
        provenance_status: j["provenance_status"].as_str().unwrap_or("unknown").into(),
        rel_l2: f("rel_l2"),
        persist_rel_l2: f("persist_rel_l2"),
        improvement_ratio: f("improvement_ratio"),
        mean_abs_error: f("mean_abs_error"),
        p95_abs_error: f("p95_abs_error"),
        max_abs_error: f("max_abs_error"),
        divergence_model_rms: f("divergence_model_rms"),
        divergence_truth_rms: f("divergence_truth_rms"),
        divergence_error_rms: f("divergence_error_rms"),
        spectrum_rel_l2: f("spectrum_rel_l2"),
        spectrum_k,
        spectrum_model,
        spectrum_truth,
    })
}

fn start(config: &EngineConfig) -> std::io::Result<(TcpStream, Child, String)> {
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/engine/reyn_engine.py");
    let mut child = Command::new(&config.python_path)
        .args([
            "-u",
            script,
            "--research-dir",
            &config.research_dir,
            "--device",
            &config.device,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let stdout = child.stdout.take().expect("piped stdout");
    let mut line = String::new();
    BufReader::new(stdout).read_line(&mut line)?;
    let json = line
        .trim()
        .strip_prefix("READY ")
        .ok_or_else(|| std::io::Error::other(format!("bad engine startup: {line}")))?;
    let ready = serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .ok_or_else(|| std::io::Error::other("invalid READY metadata"))?;
    if let Some(error) = ready["error"].as_str() {
        let _ = child.kill();
        return Err(std::io::Error::other(error.to_string()));
    }
    let device = ready["device"].as_str().unwrap_or("unknown").to_string();
    let port = ready["port"]
        .as_u64()
        .ok_or_else(|| std::io::Error::other("no port in READY"))?;
    let stream = TcpStream::connect(("127.0.0.1", port as u16))?;
    Ok((stream, child, device))
}

fn request(
    conn: &mut TcpStream,
    json: String,
    payload: &[u8],
) -> std::io::Result<(serde_json::Value, Vec<u8>)> {
    let jb = json.as_bytes();
    let body_len = 4 + jb.len() + payload.len();
    conn.write_all(&(body_len as u32).to_le_bytes())?;
    conn.write_all(&(jb.len() as u32).to_le_bytes())?;
    conn.write_all(jb)?;
    if !payload.is_empty() {
        conn.write_all(payload)?;
    }
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
                if pred(&m) {
                    return Some(m);
                }
            }
        }
        None
    }

    #[test]
    fn parses_model_library_metadata() {
        let value = serde_json::json!({
            "id": "reyn_models/h64.pth",
            "name": "h64.pth",
            "managed": true,
            "size_bytes": 1048576,
            "modified_unix": 1234,
            "checkpoint_sha256": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            "status": "clean",
            "status_detail": "fixed final epoch · source fingerprint present",
            "dimension": 2,
            "grid": 128,
            "in_channels": 4,
            "out_channels": 2,
            "max_steps": 64,
            "epoch": 40,
            "declared_epochs": 40,
            "checkpoint_role": "fixed_final",
            "scenario": "obstacle",
            "source_digest": "abc123",
            "physics_contract": "fixed_body_v2",
            "support": ["2D · 128^2 grid"],
            "limitations": ["Static body only"],
            "benchmark_report_hashes": ["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
            "unknown_fields": []
        });
        let card = parse_model_card(&value).expect("valid model card");
        assert_eq!(card.id, "reyn_models/h64.pth");
        assert_eq!(card.dimension, 2);
        assert!(card.managed);
        assert_eq!(card.source_digest.as_deref(), Some("abc123"));
        assert_eq!(card.physics_contract, "fixed_body_v2");
        assert_eq!(card.limitations, vec!["Static body only"]);
        assert!(parse_model_card(&serde_json::json!({"name": "bad"})).is_none());
    }

    #[test]
    fn parses_structured_model_rejection() {
        let value = serde_json::json!({
            "accepted": false,
            "status": "rejected",
            "summary": "unsupported checkpoint contract",
            "issues": [{
                "code": "contract.unsupported_channels",
                "field": "model_config",
                "message": "unsupported checkpoint contract",
                "severity": "error"
            }],
            "candidate": {
                "name": "bad.pth",
                "checkpoint_sha256": "abc123"
            }
        });
        let validation = parse_model_validation(&value).expect("structured validation");
        assert!(!validation.accepted);
        assert_eq!(validation.issues[0].code, "contract.unsupported_channels");
        assert_eq!(validation.candidate_name.as_deref(), Some("bad.pth"));
    }

    #[test]
    fn parses_benchmark_provenance_protocol() {
        let value = serde_json::json!({
            "ok": true,
            "seeds": [70000, 50000],
            "horizons": [1, 4],
            "rel": [[0.1, 0.2], [0.3, 0.4]],
            "persist": [[0.5, 0.6], [0.7, 0.8]],
            "global_rel": 0.25,
            "runtime_s": 1.2,
            "grid": 128,
            "epoch": 30,
            "dt_frame": 0.04,
            "provenance": {
                "verdict": "flagged",
                "training_seed": 0,
                "mixed_fork_seed": 10000,
                "mixed_fork_used": false,
                "validation_seed": 50000,
                "dataset": "standard",
                "benchmark_seeds": [
                    {"seed": 70000, "stream": "fresh_test", "overlap": false},
                    {"seed": 50000, "stream": "validation_selection", "overlap": true}
                ],
                "overlap_count": 1,
                "overlap_pct": 50.0,
                "epoch": 30,
                "declared_epochs": 30,
                "checkpoint_role": "legacy/unknown",
                "final_epoch_status": "final_epoch_role_unknown",
                "selection_metric": "val_multi_horizon_rel_l2",
                "selection_stream": "validation",
                "source_fingerprint_present": false,
                "source_fingerprint_digest": null,
                "legacy_unknown": ["checkpoint role absent", "source fingerprint absent"],
                "flags": ["benchmark seeds overlap reserved streams: 50000=validation_selection"]
            }
        });
        match parse_benchmark(&value, "model.pth".into()) {
            Msg::Benchmark(result) => {
                assert_eq!(result.seeds, vec![70000, 50000]);
                assert_eq!(result.provenance.verdict, "flagged");
                assert_eq!(result.provenance.overlap_count, 1);
                assert_eq!(
                    result.provenance.benchmark_seeds[1].stream,
                    "validation_selection"
                );
                assert!(!result.provenance.source_fingerprint_present);
                assert_eq!(result.dt_frame, 0.04);
            }
            _ => panic!("valid suite protocol did not parse"),
        }
    }

    #[test]
    fn parses_benchmark_inspector_protocol_and_rejects_short_payload() {
        let value = serde_json::json!({
            "ok": true,
            "schema": INSPECTOR_SCHEMA,
            "protocol_version": INSPECTOR_PROTOCOL_VERSION,
            "shape": [4, 3, 2, 2],
            "layout": INSPECTOR_LAYOUT,
            "variables": ["velocity", "vorticity", "pressure", "divergence"],
            "signed": [false, true, true, true],
            "units": [
                "solver_velocity_unit",
                "inverse_solver_time_unit",
                "solver_velocity_unit_squared",
                "inverse_solver_time_unit"
            ],
            "panel_sources": [
                ["MODEL", "SOLVER_REFERENCE", "DERIVED"],
                ["DERIVED_FROM_MODEL", "DERIVED_FROM_SOLVER_REFERENCE", "DERIVED"],
                ["RECOVERED_FROM_MODEL", "RECOVERED_FROM_SOLVER_REFERENCE", "DERIVED"],
                ["DERIVED_FROM_MODEL", "DERIVED_FROM_SOLVER_REFERENCE", "DERIVED"]
            ],
            "domain": INSPECTOR_DOMAIN,
            "derivative": INSPECTOR_DERIVATIVE,
            "pressure": INSPECTOR_PRESSURE,
            "seed": 70000,
            "horizon": 4,
            "seed_stream": "fresh_test",
            "provenance_status": "unknown",
            "rel_l2": 0.1,
            "persist_rel_l2": 0.4,
            "improvement_ratio": 4.0,
            "mean_abs_error": 0.02,
            "p95_abs_error": 0.05,
            "max_abs_error": 0.08,
            "divergence_model_rms": 0.001,
            "divergence_truth_rms": 0.0001,
            "divergence_error_rms": 0.0011,
            "spectrum_rel_l2": 0.12,
            "spectrum_k": [1.0, 2.0],
            "spectrum_model": [3.0, 1.0],
            "spectrum_truth": [2.8, 1.1]
        });
        let floats: Vec<f32> = (0..48).map(|number| number as f32).collect();
        let payload: Vec<u8> = floats
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        match parse_benchmark_inspector(&value, &payload) {
            Msg::BenchmarkInspector(result) => {
                assert_eq!(result.n, 2);
                let velocity = result
                    .maps
                    .get(InspectorVariable::Velocity)
                    .expect("velocity maps");
                assert_eq!(velocity.model, vec![0.0, 1.0, 2.0, 3.0]);
                assert_eq!(velocity.error, vec![8.0, 9.0, 10.0, 11.0]);
                let divergence = result
                    .maps
                    .get(InspectorVariable::Divergence)
                    .expect("spatial divergence maps");
                assert_eq!(divergence.model, vec![36.0, 37.0, 38.0, 39.0]);
                assert_eq!(divergence.error, vec![44.0, 45.0, 46.0, 47.0]);
                assert_eq!(result.spectrum_k, vec![1.0, 2.0]);
                assert_eq!(result.seed_stream, "fresh_test");
            }
            _ => panic!("valid inspector protocol did not parse"),
        }
        assert!(matches!(
            parse_benchmark_inspector(&value, &payload[..payload.len() - 4]),
            Msg::Error(_)
        ));
        let mut wrong_method = value;
        wrong_method["derivative"] = serde_json::json!("finite_difference");
        assert!(matches!(
            parse_benchmark_inspector(&wrong_method, &payload),
            Msg::Error(_)
        ));
    }

    /// End-to-end bridge test: spawns the real Python engine (needs the research
    /// venv + checkpoints) and verifies list_models + a predicted 3D field.
    #[test]
    fn engine_round_trip() {
        let h = EngineHandle::spawn();
        h.tx.send(Cmd::ListModels).unwrap();
        assert!(
            matches!(wait_for(&h, |m| matches!(m, Msg::Models(_)), 20),
            Some(Msg::Models(ref v)) if !v.is_empty()),
            "no models listed"
        );

        h.tx.send(Cmd::Predict {
            model: "flow3d_obs_v1.pth".into(),
            seed: 3,
        })
        .unwrap();
        match wait_for(&h, |m| matches!(m, Msg::Field(_) | Msg::Error(_)), 40) {
            Some(Msg::Field(f)) => {
                assert_eq!(f.shape, vec![3, 32, 32, 32]);
                assert_eq!(f.data.len(), 3 * 32 * 32 * 32);
                assert!(
                    !crate::flow::from_field(&f.shape, &f.data).is_empty(),
                    "field produced no particles"
                );
            }
            Some(Msg::Error(e)) => panic!("engine error: {e}"),
            _ => panic!("timed out waiting for the model field"),
        }
    }

    /// N3 bridge test: a 2D prediction with a solver-reference overlay returns
    /// model/reference planes, a sane reference RelL2, and a semigroup
    /// self-consistency number.
    #[test]
    fn predict2d_round_trip() {
        let h = EngineHandle::spawn();
        h.tx.send(Cmd::Predict2D {
            model: "obstacle_v2_shapes.pth".into(),
            steps: 8,
            seed: 1,
            want_truth: true,
            method: "spectral".into(),
            tolerance: 1e-5,
            boundary: "periodic".into(),
        })
        .unwrap();
        match wait_for(&h, |m| matches!(m, Msg::Field2D(_) | Msg::Error(_)), 60) {
            Some(Msg::Field2D(f)) => {
                assert_eq!(f.n, 128);
                assert_eq!(f.ai.len(), 3 * 128 * 128);
                let truth = f.truth.expect("want_truth but no truth returned");
                assert_eq!(truth.len(), 3 * 128 * 128);
                let rel = f.rel_l2.expect("no rel_l2");
                assert!(rel < 0.1, "held-out RelL2 unexpectedly high: {rel}");
                assert!(rel < f.persist.unwrap(), "AI should beat persistence");
                assert!(
                    f.semigroup.is_some(),
                    "even horizon should yield a semigroup number"
                );
                assert!(
                    f.p_residual < 1e-3,
                    "spectral recovery residual too high: {}",
                    f.p_residual
                );
            }
            Some(Msg::Error(e)) => panic!("engine error: {e}"),
            _ => panic!("timed out waiting for the 2D field"),
        }
    }

    /// N4 bridge test: a painted, natively-projected vortex-pair IC goes through
    /// the unified model (mask=0-capable) and comes back as a finite field.
    #[test]
    fn predict_ic_round_trip() {
        let mut field = crate::painter::PaintField::default();
        field.preset_vortex_pair();
        field.project(1e-8, 2000);
        assert!(field.div_max < 1e-6, "painted IC must be divergence-free");
        let ic = std::sync::Arc::new(field.ic_payload().expect("projected IC"));

        let h = EngineHandle::spawn();
        h.tx.send(Cmd::PredictIC {
            model: "obstacle_unified.pth".into(),
            steps: 4,
            ic,
        })
        .unwrap();
        match wait_for(&h, |m| matches!(m, Msg::Field2D(_) | Msg::Error(_)), 60) {
            Some(Msg::Field2D(f)) => {
                assert_eq!(f.n, 128);
                assert_eq!(f.scenario, "painted");
                assert!(f.truth.is_none(), "painted ICs have no solver truth");
                assert_eq!(f.ai.len(), 3 * 128 * 128);
                assert!(f.ai.iter().all(|v| v.is_finite()), "non-finite prediction");
                assert!(f.semigroup.is_some(), "trust signal missing");
            }
            Some(Msg::Error(e)) => panic!("engine error: {e}"),
            _ => panic!("timed out waiting for the painted-IC prediction"),
        }
    }

    /// CAD bridge test: a voxel box mask → real Brinkman warmup → 3D model →
    /// velocity + recovered pressure + the smoothed mask, all finite, with a
    /// meaningful surface-load spread.
    #[test]
    fn predict_cad_round_trip() {
        let n = 32usize;
        let mut mask = vec![0f32; n * n * n];
        for i in 8..13 {
            for j in 13..19 {
                for k in 13..19 {
                    mask[i * n * n + j * n + k] = 1.0;
                }
            }
        }
        let h = EngineHandle::spawn();
        h.tx.send(Cmd::CadPredict {
            request_id: "test-cad-request".into(),
            model: "flow3d_obs_v1.pth".into(),
            steps: 4,
            mask: std::sync::Arc::new(mask),
            reynolds: 150.0,
            characteristic_length_solver: 0.6,
            reference_length_m: 1.0,
            velocity_mps: 1.0,
            density_kg_m3: 1.225,
            reference_pressure_pa: 101_325.0,
        })
        .unwrap();
        match wait_for(&h, |m| matches!(m, Msg::CadField(_) | Msg::Error(_)), 90) {
            Some(Msg::CadField(f)) => {
                assert_eq!(f.request_id, "test-cad-request");
                assert_eq!(f.n, n);
                assert_eq!(f.vel.len(), 3 * n * n * n);
                assert_eq!(f.pressure.len(), n * n * n);
                assert_eq!(f.mask.len(), n * n * n);
                assert!(f.vel.iter().all(|v| v.is_finite()));
                let cp_min = f.cp.iter().copied().fold(f32::INFINITY, f32::min);
                let cp_max = f.cp.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                assert!(cp_max > cp_min, "no Cp spread on the body");
                let surf = crate::cad::surface_insights(&f.mask, &f.cp, n);
                assert_eq!(surf.len(), 2, "surface load + suction pins expected");
            }
            Some(Msg::Error(e)) => panic!("engine error: {e}"),
            _ => panic!("timed out waiting for the CAD prediction"),
        }
    }

    /// N5 bridge test: the suite returns a full seeds × horizons matrix with the
    /// production 2D model beating persistence in every cell.
    #[test]
    fn benchmark_round_trip() {
        let h = EngineHandle::spawn();
        h.tx.send(Cmd::RunBenchmark {
            model: "obstacle_v2_shapes.pth".into(),
            seeds: vec![70000, 70001],
            horizons: vec![1, 4],
        })
        .unwrap();
        match wait_for(&h, |m| matches!(m, Msg::Benchmark(_) | Msg::Error(_)), 120) {
            Some(Msg::Benchmark(b)) => {
                assert_eq!(b.seeds, vec![70000, 70001]);
                assert_eq!(b.horizons, vec![1, 4]);
                assert_eq!(b.rel.len(), 2);
                assert_eq!(b.rel[0].len(), 2);
                for (row_r, row_p) in b.rel.iter().zip(&b.persist) {
                    for (r, p) in row_r.iter().zip(row_p) {
                        assert!(r.is_finite() && *r > 0.0);
                        assert!(r < p, "model should beat persistence ({r} vs {p})");
                    }
                }
                assert!(b.global_rel > 0.0 && b.runtime_s > 0.0);
                assert_eq!(b.provenance.overlap_count, 0);
                assert!(b
                    .provenance
                    .benchmark_seeds
                    .iter()
                    .all(|seed| seed.stream == "fresh_test" && !seed.overlap));
            }
            Some(Msg::Error(e)) => panic!("engine error: {e}"),
            _ => panic!("timed out waiting for the benchmark"),
        }

        h.tx.send(Cmd::InspectBenchmarkCell {
            model: "obstacle_v2_shapes.pth".into(),
            seed: 70000,
            horizon: 4,
        })
        .unwrap();
        match wait_for(
            &h,
            |m| matches!(m, Msg::BenchmarkInspector(_) | Msg::Error(_)),
            60,
        ) {
            Some(Msg::BenchmarkInspector(cell)) => {
                assert_eq!((cell.seed, cell.horizon, cell.n), (70000, 4, 128));
                for variable in InspectorVariable::ALL {
                    let maps = cell.maps.get(variable).expect("all inspector modes");
                    assert_eq!(maps.model.len(), 128 * 128);
                    assert_eq!(maps.reference.len(), 128 * 128);
                    assert_eq!(maps.error.len(), 128 * 128);
                }
                assert_eq!(cell.seed_stream, "fresh_test");
                assert!(cell.rel_l2 > 0.0 && cell.rel_l2 < cell.persist_rel_l2);
                assert!(cell.improvement_ratio > 1.0);
                assert!(cell.spectrum_k.len() > 10);
                assert_eq!(cell.spectrum_k.len(), cell.spectrum_model.len());
                assert_eq!(cell.spectrum_k.len(), cell.spectrum_truth.len());
                assert!(cell.spectrum_rel_l2.is_finite());
            }
            Some(Msg::Error(e)) => panic!("engine error: {e}"),
            _ => panic!("timed out waiting for benchmark inspector"),
        }
    }
}
