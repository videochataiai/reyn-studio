//! Deterministic N5 benchmark report rendering.
//!
//! PNG and PDF are presentation artifacts derived from the already-hashed
//! canonical JSON report. They carry the canonical payload digest and an
//! explicit `UNSIGNED` status, but never claim cryptographic authenticity.

use flate2::{write::ZlibEncoder, Compression};
use fontdue::{Font, FontSettings};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fmt;
use std::io::Write;

use crate::signing::{
    self, SignedEvidenceArtifact, VerificationOutcome, VerificationPolicy, VerificationStatus,
    SIGNATURE_ALGORITHM,
};

pub const REPORT_CARD_SCHEMA: &str = "reyn.benchmark-report-card.v1";
pub const PNG_WIDTH: u32 = 1_800;
const MIN_PNG_HEIGHT: u32 = 3_000;
const MAX_REPORT_BYTES: usize = 2 * 1024 * 1024;

const PAPER: [u8; 3] = [248, 246, 243];
const INK: [u8; 3] = [41, 29, 22];
const MUTED: [u8; 3] = [103, 82, 71];
const HAIRLINE: [u8; 3] = [205, 193, 185];
const EMBER: [u8; 3] = [194, 79, 8];
const GOLD: [u8; 3] = [145, 101, 0];
const BLUE: [u8; 3] = [24, 91, 135];
const RED: [u8; 3] = [157, 48, 40];
const GREEN: [u8; 3] = [35, 112, 79];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportFormat {
    Png,
    Pdf,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Pdf => "pdf",
        }
    }

    pub fn media_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Pdf => "application/pdf",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExportArtifact {
    pub format: ExportFormat,
    pub bytes: Vec<u8>,
    pub canonical_payload_sha256: String,
    pub content_sha256: String,
    pub signature_sidecar_sha256: Option<String>,
}

impl ExportArtifact {
    pub fn media_type(&self) -> &'static str {
        self.format.media_type()
    }

    /// Recompute the artifact digest and confirm that the rendered file embeds
    /// the same canonical-payload digest and authenticity status.
    pub fn verify(&self) -> Result<(), ExportError> {
        let actual = sha256_hex(&self.bytes);
        if actual != self.content_sha256 {
            return Err(ExportError::Integrity(
                "rendered artifact bytes do not match content_sha256".into(),
            ));
        }
        let bytes = &self.bytes;
        let hash = self.canonical_payload_sha256.as_bytes();
        if !contains_bytes(bytes, hash) {
            return Err(ExportError::Integrity(
                "rendered artifact does not carry the canonical payload hash".into(),
            ));
        }
        if let Some(sidecar_sha256) = &self.signature_sidecar_sha256 {
            if !contains_bytes(bytes, b"SIGNED PAYLOAD")
                || !contains_bytes(bytes, sidecar_sha256.as_bytes())
            {
                return Err(ExportError::Integrity(
                    "rendered artifact does not carry its signed-payload sidecar lineage".into(),
                ));
            }
        } else if !contains_bytes(bytes, b"UNSIGNED") {
            return Err(ExportError::Integrity(
                "rendered artifact does not carry its unsigned status".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum ExportError {
    InvalidReport(String),
    Integrity(String),
    Render(String),
}

impl fmt::Display for ExportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidReport(message) => write!(f, "invalid benchmark report: {message}"),
            Self::Integrity(message) => write!(f, "benchmark report integrity failed: {message}"),
            Self::Render(message) => write!(f, "benchmark report rendering failed: {message}"),
        }
    }
}

impl std::error::Error for ExportError {}

#[derive(Clone, Debug)]
struct VerifiedReport {
    payload: Value,
    canonical_payload_sha256: String,
}

#[derive(Clone, Debug)]
struct VerifiedSignatureMark {
    key_id: String,
    key_fingerprint_sha256: String,
    signature_sidecar_sha256: String,
}

#[derive(Clone, Debug)]
struct MetricRow {
    seed: u64,
    horizon: u64,
    rel_l2: f64,
    persistence_rel_l2: f64,
}

#[derive(Clone, Debug)]
struct VariableRow {
    label: String,
    unit: String,
    signed: bool,
    model_source: String,
    reference_source: String,
    error_source: String,
    comparison_scale: f64,
    error_scale: f64,
    method: String,
}

pub fn export_report(
    canonical_report_json: &str,
    format: ExportFormat,
) -> Result<ExportArtifact, ExportError> {
    let report = verify_report(canonical_report_json)?;
    let canvas = render_report(&report, None)?;
    let bytes = match format {
        ExportFormat::Png => encode_png(&canvas, &report.canonical_payload_sha256, None)?,
        ExportFormat::Pdf => encode_pdf(&canvas, &report.canonical_payload_sha256, None)?,
    };
    let artifact = ExportArtifact {
        format,
        content_sha256: sha256_hex(&bytes),
        canonical_payload_sha256: report.canonical_payload_sha256,
        signature_sidecar_sha256: None,
        bytes,
    };
    artifact.verify()?;
    Ok(artifact)
}

/// Render a report whose immutable payload has a detached, self-verifying
/// Ed25519 sidecar. The visual artifact carries the sidecar digest and key
/// fingerprint; organization trust still requires an independently distributed
/// fingerprint.
pub fn export_report_with_signature(
    canonical_report_json: &str,
    format: ExportFormat,
    signature: &SignedEvidenceArtifact,
) -> Result<ExportArtifact, ExportError> {
    let report = verify_report(canonical_report_json)?;
    let outcome = verify_report_signature(
        canonical_report_json,
        signature,
        &VerificationPolicy::portable_untrusted(),
    );
    if outcome.status != VerificationStatus::ValidUntrustedKey {
        return Err(ExportError::Integrity(format!(
            "detached signature is not valid for this report: {:?} ({})",
            outcome.status, outcome.detail
        )));
    }
    let mark = VerifiedSignatureMark {
        key_id: signature.authenticity.key_id.clone(),
        key_fingerprint_sha256: signature.authenticity.key_fingerprint_sha256.clone(),
        signature_sidecar_sha256: signature
            .content_sha256()
            .map_err(|error| ExportError::Integrity(error.to_string()))?,
    };
    let canvas = render_report(&report, Some(&mark))?;
    let bytes = match format {
        ExportFormat::Png => encode_png(&canvas, &report.canonical_payload_sha256, Some(&mark))?,
        ExportFormat::Pdf => encode_pdf(&canvas, &report.canonical_payload_sha256, Some(&mark))?,
    };
    let artifact = ExportArtifact {
        format,
        content_sha256: sha256_hex(&bytes),
        canonical_payload_sha256: report.canonical_payload_sha256,
        signature_sidecar_sha256: Some(mark.signature_sidecar_sha256),
        bytes,
    };
    artifact.verify()?;
    Ok(artifact)
}

/// Verify the JSON integrity record without treating it as a signature.
pub fn verify_canonical_report(canonical_report_json: &str) -> Result<String, ExportError> {
    verify_report(canonical_report_json).map(|report| report.canonical_payload_sha256)
}

/// Verify a detached Ed25519 sidecar against the exact canonical report bytes
/// and report identity. A valid self-contained key remains untrusted until its
/// fingerprint is supplied through `policy`.
pub fn verify_report_signature(
    canonical_report_json: &str,
    artifact: &SignedEvidenceArtifact,
    policy: &VerificationPolicy,
) -> VerificationOutcome {
    let report = match verify_report(canonical_report_json) {
        Ok(report) => report,
        Err(error) => {
            return VerificationOutcome {
                status: match error {
                    ExportError::Integrity(_) => VerificationStatus::HashMismatch,
                    ExportError::InvalidReport(_) | ExportError::Render(_) => {
                        VerificationStatus::Malformed
                    }
                },
                key_id: Some(artifact.authenticity.key_id.clone()),
                key_fingerprint_sha256: Some(artifact.authenticity.key_fingerprint_sha256.clone()),
                detail: format!("canonical report verification failed: {error}"),
            };
        }
    };
    let run_id = report
        .payload
        .get("run_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let report_schema = report
        .payload
        .get("report_schema")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if artifact.source.run_id != run_id || artifact.source.report_schema != report_schema {
        return VerificationOutcome {
            status: VerificationStatus::HashMismatch,
            key_id: Some(artifact.authenticity.key_id.clone()),
            key_fingerprint_sha256: Some(artifact.authenticity.key_fingerprint_sha256.clone()),
            detail: "signature source identity does not match report_schema and run_id".into(),
        };
    }
    signing::verify_signed_hash(
        &report.canonical_payload_sha256,
        &signing::sha256_hex(canonical_report_json.as_bytes()),
        artifact,
        policy,
    )
}

fn verify_report(canonical_report_json: &str) -> Result<VerifiedReport, ExportError> {
    if canonical_report_json.len() > MAX_REPORT_BYTES {
        return Err(ExportError::InvalidReport(
            "report exceeds the 2 MiB rendering limit".into(),
        ));
    }
    let mut payload: Value = serde_json::from_str(canonical_report_json)
        .map_err(|error| ExportError::InvalidReport(error.to_string()))?;
    let object = payload
        .as_object_mut()
        .ok_or_else(|| ExportError::InvalidReport("root must be an object".into()))?;
    let embedded_hash = object
        .remove("integrity_sha256")
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| ExportError::InvalidReport("integrity_sha256 is required".into()))?;
    if !is_sha256(&embedded_hash) {
        return Err(ExportError::InvalidReport(
            "integrity_sha256 must be 64 lowercase hexadecimal characters".into(),
        ));
    }
    let canonical = serde_json::to_vec(&payload)
        .map_err(|error| ExportError::InvalidReport(error.to_string()))?;
    let actual_hash = sha256_hex(&canonical);
    if actual_hash != embedded_hash {
        return Err(ExportError::Integrity(format!(
            "expected {embedded_hash}, recomputed {actual_hash}"
        )));
    }

    require_text(&payload, "report_schema", Some(REPORT_CARD_SCHEMA))?;
    require_text(&payload, "run_id", None)?;
    require_text(&payload, "protocol_id", None)?;
    require_text(&payload, "model", None)?;
    require_text(&payload, "model_checkpoint_sha256", None)?;
    require_text(&payload, "protocol", None)?;
    require_text(&payload, "canonicalization", None)?;
    require_finite_number(&payload, "global_rel_l2")?;
    require_finite_number(&payload, "runtime_s")?;
    let run_id = payload["run_id"]
        .as_str()
        .expect("run_id was validated as text");
    if uuid::Uuid::parse_str(run_id).is_err() {
        return Err(ExportError::InvalidReport(
            "run_id must be a stable UUID".into(),
        ));
    }
    if !payload["model_checkpoint_sha256"]
        .as_str()
        .is_some_and(is_sha256)
    {
        return Err(ExportError::InvalidReport(
            "model_checkpoint_sha256 must be a lowercase SHA-256 digest".into(),
        ));
    }
    if payload.get("integrity_algorithm").and_then(Value::as_str) != Some("SHA-256") {
        return Err(ExportError::InvalidReport(
            "integrity_algorithm must be SHA-256".into(),
        ));
    }

    let authenticity = payload
        .get("authenticity")
        .and_then(Value::as_object)
        .ok_or_else(|| ExportError::InvalidReport("authenticity record is required".into()))?;
    if authenticity.get("status").and_then(Value::as_str) != Some("UNSIGNED")
        || !authenticity.get("signature").is_some_and(Value::is_null)
    {
        return Err(ExportError::InvalidReport(
            "this exporter only renders explicitly UNSIGNED reports; signed claims require a separate verified organization-key layer".into(),
        ));
    }

    metric_rows(&payload)?;
    variable_rows(&payload)?;
    validate_provenance(&payload)?;

    Ok(VerifiedReport {
        payload,
        canonical_payload_sha256: embedded_hash,
    })
}

fn validate_provenance(payload: &Value) -> Result<(), ExportError> {
    let provenance = payload
        .get("provenance")
        .and_then(Value::as_object)
        .ok_or_else(|| ExportError::InvalidReport("provenance record is required".into()))?;
    let verdict = provenance
        .get("verdict")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    if !matches!(verdict, "clean" | "flagged" | "unknown") {
        return Err(ExportError::InvalidReport(
            "provenance verdict must be clean, flagged, or unknown".into(),
        ));
    }
    let proposition = provenance
        .get("checked_proposition")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ExportError::InvalidReport("provenance checked_proposition is required".into())
        })?;
    if verdict == "clean" && proposition != "no collision in checked RNG streams" {
        return Err(ExportError::InvalidReport(
            "clean provenance must state the bounded RNG-stream proposition".into(),
        ));
    }
    if provenance
        .get("validation_is_independent_test")
        .and_then(Value::as_bool)
        != Some(false)
    {
        return Err(ExportError::InvalidReport(
            "validation/checkpoint-selection evidence cannot be labeled independent test data"
                .into(),
        ));
    }
    Ok(())
}

fn metric_rows(payload: &Value) -> Result<Vec<MetricRow>, ExportError> {
    let seeds = number_array(payload, "seeds")?;
    let horizons = number_array(payload, "horizons")?;
    let rel = number_matrix(payload, "rel_l2", seeds.len(), horizons.len())?;
    let persistence = number_matrix(payload, "persistence", seeds.len(), horizons.len())?;
    if seeds.is_empty() || horizons.is_empty() {
        return Err(ExportError::InvalidReport(
            "seed and horizon arrays cannot be empty".into(),
        ));
    }
    if seeds.len() > 64 || horizons.len() > 64 {
        return Err(ExportError::InvalidReport(
            "seed/horizon matrix is too large for a report card".into(),
        ));
    }
    let mut rows = Vec::with_capacity(seeds.len() * horizons.len());
    for (seed_index, seed) in seeds.iter().enumerate() {
        for (horizon_index, horizon) in horizons.iter().enumerate() {
            rows.push(MetricRow {
                seed: *seed,
                horizon: *horizon,
                rel_l2: rel[seed_index][horizon_index],
                persistence_rel_l2: persistence[seed_index][horizon_index],
            });
        }
    }
    Ok(rows)
}

fn variable_rows(payload: &Value) -> Result<Vec<VariableRow>, ExportError> {
    let Some(selected) = payload.get("selected_cell_evidence") else {
        return Ok(Vec::new());
    };
    if selected.is_null() {
        return Ok(Vec::new());
    }
    let variables = selected
        .pointer("/spatial_variable_evidence/variables")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ExportError::InvalidReport(
                "selected cell must include spatial variable evidence".into(),
            )
        })?;
    if variables.len() != 4 {
        return Err(ExportError::InvalidReport(
            "selected cell must describe four inspector variables".into(),
        ));
    }
    let mut rows = Vec::with_capacity(variables.len());
    for variable in variables {
        let get = |key: &str| {
            variable.get(key).and_then(Value::as_str).ok_or_else(|| {
                ExportError::InvalidReport(format!("inspector variable {key} is required"))
            })
        };
        let number = |key: &str| {
            variable
                .get(key)
                .and_then(Value::as_f64)
                .filter(|value| value.is_finite() && *value > 0.0)
                .ok_or_else(|| {
                    ExportError::InvalidReport(format!(
                        "inspector variable {key} must be finite and positive"
                    ))
                })
        };
        let sources = variable
            .get("sources")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                ExportError::InvalidReport("inspector variable sources are required".into())
            })?;
        let source = |key: &str| {
            sources
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| {
                    ExportError::InvalidReport(format!(
                        "inspector variable source {key} is required"
                    ))
                })
        };
        let model_source = source("model")?;
        let reference_source = source("solver_reference")?;
        let error_source = source("error")?;
        if !matches!(
            model_source.as_str(),
            "MODEL" | "DERIVED_FROM_MODEL" | "RECOVERED_FROM_MODEL"
        ) || !matches!(
            reference_source.as_str(),
            "SOLVER_REFERENCE"
                | "DERIVED_FROM_SOLVER_REFERENCE"
                | "RECOVERED_FROM_SOLVER_REFERENCE"
        ) || error_source != "DERIVED"
        {
            return Err(ExportError::InvalidReport(
                "inspector source classes are inconsistent".into(),
            ));
        }
        rows.push(VariableRow {
            label: get("label")?.to_owned(),
            unit: get("unit")?.to_owned(),
            signed: variable
                .get("signed")
                .and_then(Value::as_bool)
                .ok_or_else(|| {
                    ExportError::InvalidReport(
                        "inspector variable signed status is required".into(),
                    )
                })?,
            model_source,
            reference_source,
            error_source,
            comparison_scale: number("comparison_scale")?,
            error_scale: number("error_scale")?,
            method: get("method")?.to_owned(),
        });
    }
    Ok(rows)
}

fn render_report(
    report: &VerifiedReport,
    signature: Option<&VerifiedSignatureMark>,
) -> Result<Canvas, ExportError> {
    let metrics = metric_rows(&report.payload)?;
    let variables = variable_rows(&report.payload)?;
    let warnings = report_warnings(&report.payload, signature.is_some());
    let extra_metric_rows = metrics.len().saturating_sub(8) as u32;
    let extra_warnings = warnings.len().saturating_sub(4) as u32;
    let height = MIN_PNG_HEIGHT
        .saturating_add(extra_metric_rows * 38)
        .saturating_add(extra_warnings * 72)
        .saturating_add(if signature.is_some() { 180 } else { 0 });
    let mut canvas = Canvas::new(PNG_WIDTH, height, PAPER);
    let fonts = Fonts::load()?;
    let left = 96;
    let right = PNG_WIDTH as i32 - 96;
    let width = right - left;
    let mut y = 82;

    canvas.text(&fonts.inter, "Reyn Studio", 25.0, left, y, EMBER);
    canvas.text(
        &fonts.inter,
        "BENCHMARK EVIDENCE REPORT",
        18.0,
        left + 180,
        y + 4,
        MUTED,
    );
    if signature.is_some() {
        canvas.stroke_rect(right - 284, y - 7, 284, 44, GREEN, 2);
        canvas.text(
            &fonts.mono,
            "SIGNED PAYLOAD",
            18.0,
            right - 258,
            y + 2,
            GREEN,
        );
    } else {
        canvas.stroke_rect(right - 184, y - 7, 184, 44, GOLD, 2);
        canvas.text(&fonts.mono, "UNSIGNED", 18.0, right - 160, y + 2, GOLD);
    }
    y += 72;
    canvas.text(
        &fonts.inter,
        if signature.is_some() {
            "Model qualification with detached authenticity evidence"
        } else {
            "Model qualification, without implied authenticity"
        },
        42.0,
        left,
        y,
        INK,
    );
    y += 68;
    canvas.text(
        &fonts.inter,
        "Prediction, solver reference, recovered quantities, derivations, provenance, and integrity remain distinct.",
        19.0,
        left,
        y,
        MUTED,
    );
    y += 58;
    canvas.line(left, y, right, y, HAIRLINE, 2);
    y += 34;

    let run_id = text(&report.payload, "run_id");
    let protocol_id = text(&report.payload, "protocol_id");
    let model = text(&report.payload, "model");
    let model_checkpoint_sha256 = text(&report.payload, "model_checkpoint_sha256");
    let grid = integer_text(&report.payload, "grid");
    let epoch = integer_text(&report.payload, "epoch");
    let runtime = number(&report.payload, "runtime_s");
    let global_rel = number(&report.payload, "global_rel_l2");
    canvas.label_value(&fonts, left, y, "RUN ID", &run_id, width / 2 - 24);
    canvas.label_value(
        &fonts,
        left + width / 2,
        y,
        "PROTOCOL",
        &protocol_id,
        width / 2,
    );
    y += 78;
    canvas.label_value(&fonts, left, y, "MODEL", &model, width / 2 - 24);
    canvas.label_value(
        &fonts,
        left + width / 2,
        y,
        "GRID / EPOCH",
        &format!("{grid} x {grid} / {epoch}"),
        width / 2,
    );
    y += 78;
    canvas.label_value(
        &fonts,
        left,
        y,
        "MODEL CHECKPOINT SHA-256",
        &model_checkpoint_sha256,
        width,
    );
    y += 78;
    canvas.label_value(
        &fonts,
        left,
        y,
        "GLOBAL MODEL REL L2",
        &format_scientific(global_rel),
        width / 2 - 24,
    );
    canvas.label_value(
        &fonts,
        left + width / 2,
        y,
        "SUITE RUNTIME",
        &format!("{runtime:.3} s"),
        width / 2,
    );
    y += 100;

    y = section(
        &mut canvas,
        &fonts,
        left,
        right,
        y,
        "Seed x horizon metrics",
    );
    let columns = [left, left + 210, left + 400, left + 710, left + 1_080];
    let headers = [
        "SEED",
        "HORIZON",
        "MODEL REL L2",
        "PERSISTENCE REL L2",
        "IMPROVEMENT",
    ];
    for (column, header) in columns.iter().zip(headers) {
        canvas.text(&fonts.mono, header, 15.0, *column, y, MUTED);
    }
    y += 34;
    canvas.line(left, y, right, y, HAIRLINE, 1);
    y += 12;
    for row in &metrics {
        let improvement = row.persistence_rel_l2 / row.rel_l2.max(1e-12);
        let values = [
            row.seed.to_string(),
            format!("{}x", row.horizon),
            format_scientific(row.rel_l2),
            format_scientific(row.persistence_rel_l2),
            format!("{improvement:.3}x"),
        ];
        for (column, value) in columns.iter().zip(values) {
            canvas.text(&fonts.mono, &value, 17.0, *column, y, INK);
        }
        y += 34;
    }
    y += 34;

    y = section(&mut canvas, &fonts, left, right, y, "Provenance");
    let provenance = report.payload.get("provenance").unwrap_or(&Value::Null);
    let verdict = text(provenance, "verdict").to_ascii_uppercase();
    let proposition = text(provenance, "checked_proposition");
    let verdict_color = match verdict.as_str() {
        "CLEAN" => GREEN,
        "FLAGGED" => RED,
        _ => GOLD,
    };
    canvas.text(&fonts.mono, &verdict, 22.0, left, y, verdict_color);
    canvas.wrapped_text(
        &fonts.inter,
        &proposition,
        18.0,
        left + 170,
        y,
        right - left - 170,
        INK,
        26,
    );
    y += 54;
    let dataset = text(provenance, "dataset");
    let role = text(provenance, "checkpoint_role");
    let selection = text(provenance, "selection_stream");
    canvas.label_value(&fonts, left, y, "DATASET", &dataset, width / 3 - 18);
    canvas.label_value(
        &fonts,
        left + width / 3,
        y,
        "CHECKPOINT ROLE",
        &role,
        width / 3 - 18,
    );
    canvas.label_value(
        &fonts,
        left + 2 * width / 3,
        y,
        "SELECTION STREAM",
        &selection,
        width / 3,
    );
    y += 86;
    canvas.wrapped_text(
        &fonts.inter,
        "Validation/checkpoint-selection data is not independent test evidence. Stream non-collision is not field-space or trajectory non-overlap.",
        17.0,
        left,
        y,
        width,
        MUTED,
        25,
    );
    y += 78;

    y = section(
        &mut canvas,
        &fonts,
        left,
        right,
        y,
        "Selected-cell inspector methodology",
    );
    let selected = report.payload.get("selected_cell_evidence");
    if variables.is_empty() {
        canvas.text(
            &fonts.inter,
            "No selected-cell maps were attached to this canonical report.",
            18.0,
            left,
            y,
            GOLD,
        );
        y += 58;
    } else {
        let selected = selected.unwrap_or(&Value::Null);
        canvas.text(
            &fonts.mono,
            &format!(
                "SEED {}  /  HORIZON {}x  /  MODEL REL L2 {}  /  PERSISTENCE {}",
                integer_text(selected, "seed"),
                integer_text(selected, "horizon"),
                format_scientific(number(selected, "rel_l2")),
                format_scientific(number(selected, "persistence_rel_l2"))
            ),
            16.0,
            left,
            y,
            BLUE,
        );
        y += 44;
        for variable in &variables {
            canvas.line(left, y, right, y, HAIRLINE, 1);
            y += 18;
            canvas.text(&fonts.inter, &variable.label, 20.0, left, y, INK);
            canvas.text(
                &fonts.mono,
                &format!(
                    "{}  /  {}",
                    variable.unit,
                    if variable.signed {
                        "SIGNED SCALE +/-"
                    } else {
                        "UNSIGNED SCALE 0..MAX"
                    }
                ),
                14.0,
                left + 360,
                y + 3,
                MUTED,
            );
            y += 34;
            canvas.text(
                &fonts.mono,
                &format!(
                    "MODEL: {}   REFERENCE: {}   ERROR: {}",
                    variable.model_source, variable.reference_source, variable.error_source
                ),
                14.0,
                left,
                y,
                BLUE,
            );
            y += 29;
            canvas.text(
                &fonts.mono,
                &format!(
                    "SHARED MODEL/REFERENCE SCALE {}   ERROR SCALE {}",
                    format_scientific(variable.comparison_scale),
                    format_scientific(variable.error_scale)
                ),
                14.0,
                left,
                y,
                MUTED,
            );
            y += 29;
            let lines = canvas.wrapped_text(
                &fonts.inter,
                &variable.method,
                15.0,
                left,
                y,
                width,
                MUTED,
                23,
            );
            y += 22 * lines as i32 + 14;
        }
        y += 20;
    }

    y = section(
        &mut canvas,
        &fonts,
        left,
        right,
        y,
        "Warnings and limitations",
    );
    for warning in &warnings {
        canvas.text(&fonts.mono, "!", 17.0, left, y, GOLD);
        let lines = canvas.wrapped_text(
            &fonts.inter,
            warning,
            16.0,
            left + 32,
            y,
            width - 32,
            INK,
            24,
        );
        y += 24 * lines as i32 + 10;
    }
    y += 22;

    y = section(
        &mut canvas,
        &fonts,
        left,
        right,
        y,
        "Integrity verification",
    );
    canvas.text(
        &fonts.mono,
        "CANONICAL JSON PAYLOAD SHA-256",
        15.0,
        left,
        y,
        MUTED,
    );
    y += 30;
    canvas.text(
        &fonts.mono,
        &report.canonical_payload_sha256,
        19.0,
        left,
        y,
        INK,
    );
    y += 46;
    let mut instructions = vec![
        "1. Keep the canonical JSON report beside this PNG/PDF.",
        "2. Parse JSON, remove only integrity_sha256, and serialize compact UTF-8 JSON with object keys in lexical order.",
        "3. Compute SHA-256 over those exact canonical bytes; it must equal the digest above.",
        "4. Separately hash this PNG/PDF if artifact-byte integrity is required.",
    ];
    if signature.is_some() {
        instructions.push(
            "5. Verify the detached Ed25519 sidecar over the raw 32-byte canonical digest; then compare its key fingerprint through an independent trusted channel.",
        );
    } else {
        instructions.push(
            "5. This rendering is UNSIGNED. SHA-256 and a PNG/PDF file do not establish author or organization authenticity.",
        );
    }
    for instruction in instructions {
        let lines = canvas.wrapped_text(
            &fonts.inter,
            instruction,
            16.0,
            left,
            y,
            width,
            if instruction.starts_with("5.") {
                if signature.is_some() {
                    GREEN
                } else {
                    RED
                }
            } else {
                MUTED
            },
            24,
        );
        y += 24 * lines as i32 + 8;
    }

    if let Some(signature) = signature {
        y += 18;
        y = section(
            &mut canvas,
            &fonts,
            left,
            right,
            y,
            "Detached authenticity sidecar",
        );
        canvas.label_value(
            &fonts,
            left,
            y,
            "ALGORITHM / KEY ID",
            &format!("Ed25519 / {}", signature.key_id),
            width,
        );
        y += 74;
        canvas.label_value(
            &fonts,
            left,
            y,
            "KEY FINGERPRINT SHA-256",
            &signature.key_fingerprint_sha256,
            width,
        );
        y += 74;
        canvas.label_value(
            &fonts,
            left,
            y,
            "SIGNATURE SIDECAR SHA-256",
            &signature.signature_sidecar_sha256,
            width,
        );
        y += 70;
    }

    let footer_y = canvas.height as i32 - 68;
    canvas.line(left, footer_y - 20, right, footer_y - 20, HAIRLINE, 1);
    canvas.text(
        &fonts.inter,
        "Generated deterministically from the canonical JSON payload.",
        14.0,
        left,
        footer_y,
        MUTED,
    );
    canvas.text(
        &fonts.mono,
        REPORT_CARD_SCHEMA,
        14.0,
        right - 330,
        footer_y,
        MUTED,
    );

    if y > footer_y - 22 {
        return Err(ExportError::Render(
            "report content exceeded the deterministic page bounds".into(),
        ));
    }
    Ok(canvas)
}

fn section(canvas: &mut Canvas, fonts: &Fonts, left: i32, right: i32, y: i32, title: &str) -> i32 {
    canvas.text(&fonts.inter, title, 25.0, left, y, INK);
    canvas.line(left, y + 42, right, y + 42, HAIRLINE, 2);
    y + 68
}

fn report_warnings(payload: &Value, signed_payload: bool) -> Vec<String> {
    let mut warnings = Vec::new();
    for pointer in [
        "/warnings",
        "/provenance/flags",
        "/provenance/legacy_unknown",
        "/limitations",
    ] {
        if let Some(values) = payload.pointer(pointer).and_then(Value::as_array) {
            for value in values {
                if let Some(message) = value.as_str() {
                    warnings.push(message.to_owned());
                }
            }
        }
    }
    if payload
        .get("limitations")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        warnings.extend([
            "A named numerical solver output is a solver reference, not automatically physical truth."
                .into(),
            "Recovered pressure is density-normalized and is not physical Cp without a recorded reference state."
                .into(),
            "Consistency, provenance, integrity, and independent accuracy/validation are separate claims."
                .into(),
        ]);
    }
    if signed_payload {
        warnings.push(
            "The detached Ed25519 signature verifies this canonical payload; organization identity still requires an independently trusted fingerprint and current revocation information."
                .into(),
        );
    } else if !warnings
        .iter()
        .any(|warning| warning.contains("UNSIGNED") || warning.contains("authenticity"))
    {
        warnings
            .push("This PNG/PDF is UNSIGNED and does not imply cryptographic authenticity.".into());
    }
    warnings.sort();
    warnings.dedup();
    warnings
}

fn encode_png(
    canvas: &Canvas,
    canonical_hash: &str,
    signature: Option<&VerifiedSignatureMark>,
) -> Result<Vec<u8>, ExportError> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, canvas.width, canvas.height);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::High);
        encoder.set_filter(png::Filter::Sub);
        encoder
            .add_text_chunk("ReynCanonicalPayloadSHA256".into(), canonical_hash.into())
            .map_err(|error| ExportError::Render(error.to_string()))?;
        if let Some(signature) = signature {
            for (key, value) in [
                ("ReynAuthenticityStatus", "SIGNED PAYLOAD"),
                ("ReynSignatureAlgorithm", SIGNATURE_ALGORITHM),
                ("ReynSignatureKeyID", signature.key_id.as_str()),
                (
                    "ReynSignatureKeyFingerprintSHA256",
                    signature.key_fingerprint_sha256.as_str(),
                ),
                (
                    "ReynSignatureSidecarSHA256",
                    signature.signature_sidecar_sha256.as_str(),
                ),
            ] {
                encoder
                    .add_text_chunk(key.into(), value.into())
                    .map_err(|error| ExportError::Render(error.to_string()))?;
            }
        } else {
            encoder
                .add_text_chunk("ReynAuthenticityStatus".into(), "UNSIGNED".into())
                .map_err(|error| ExportError::Render(error.to_string()))?;
        }
        encoder
            .add_text_chunk("ReynReportSchema".into(), REPORT_CARD_SCHEMA.into())
            .map_err(|error| ExportError::Render(error.to_string()))?;
        let mut writer = encoder
            .write_header()
            .map_err(|error| ExportError::Render(error.to_string()))?;
        writer
            .write_image_data(&canvas.pixels)
            .map_err(|error| ExportError::Render(error.to_string()))?;
    }
    Ok(bytes)
}

fn encode_pdf(
    canvas: &Canvas,
    canonical_hash: &str,
    signature: Option<&VerifiedSignatureMark>,
) -> Result<Vec<u8>, ExportError> {
    let mut compressed = ZlibEncoder::new(Vec::new(), Compression::best());
    compressed
        .write_all(&canvas.pixels)
        .map_err(|error| ExportError::Render(error.to_string()))?;
    let compressed = compressed
        .finish()
        .map_err(|error| ExportError::Render(error.to_string()))?;

    const PAGE_W: f64 = 595.0;
    const PAGE_H: f64 = 842.0;
    let source_ratio = canvas.width as f64 / canvas.height as f64;
    let page_ratio = PAGE_W / PAGE_H;
    let (draw_w, draw_h) = if source_ratio > page_ratio {
        (PAGE_W, PAGE_W / source_ratio)
    } else {
        (PAGE_H * source_ratio, PAGE_H)
    };
    let draw_x = (PAGE_W - draw_w) / 2.0;
    let draw_y = (PAGE_H - draw_h) / 2.0;
    let content =
        format!("q\n{draw_w:.4} 0 0 {draw_h:.4} {draw_x:.4} {draw_y:.4} cm\n/Report Do\nQ\n");
    let authenticity = signature.map_or_else(
        || "UNSIGNED".to_string(),
        |signature| {
            format!(
                "SIGNED PAYLOAD; Ed25519; Key fingerprint SHA-256: {}; Signature sidecar SHA-256: {}",
                signature.key_fingerprint_sha256, signature.signature_sidecar_sha256
            )
        },
    );
    let info = format!(
        "<< /Title (Reyn Studio Benchmark Evidence Report) /Creator (Reyn Studio) /Subject (Canonical payload SHA-256: {canonical_hash}; Authenticity: {authenticity}) >>"
    );
    let objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_W:.0} {PAGE_H:.0}] /Resources << /XObject << /Report 4 0 R >> >> /Contents 5 0 R >>"
        )
        .into_bytes(),
        stream_object(
            format!(
                "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /Length {} >>",
                canvas.width,
                canvas.height,
                compressed.len()
            ),
            &compressed,
        ),
        stream_object(
            format!("<< /Length {} >>", content.len()),
            content.as_bytes(),
        ),
        info.into_bytes(),
    ];

    let signature_comments = signature.map_or_else(
        || "% ReynAuthenticityStatus: UNSIGNED\n".to_string(),
        |signature| {
            format!(
                "% ReynAuthenticityStatus: SIGNED PAYLOAD\n% ReynSignatureAlgorithm: Ed25519\n% ReynSignatureKeyID: {}\n% ReynSignatureKeyFingerprintSHA256: {}\n% ReynSignatureSidecarSHA256: {}\n",
                signature.key_id,
                signature.key_fingerprint_sha256,
                signature.signature_sidecar_sha256
            )
        },
    );
    let mut pdf =
        format!("%PDF-1.4\n% ReynCanonicalPayloadSHA256: {canonical_hash}\n{signature_comments}")
            .into_bytes();
    let mut offsets = Vec::with_capacity(objects.len());
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        write!(&mut pdf, "{} 0 obj\n", index + 1)
            .map_err(|error| ExportError::Render(error.to_string()))?;
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    write!(
        &mut pdf,
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len() + 1
    )
    .map_err(|error| ExportError::Render(error.to_string()))?;
    for offset in offsets {
        writeln!(&mut pdf, "{offset:010} 00000 n ")
            .map_err(|error| ExportError::Render(error.to_string()))?;
    }
    let file_id = sha256_hex(canonical_hash.as_bytes());
    write!(
        &mut pdf,
        "trailer\n<< /Size {} /Root 1 0 R /Info 6 0 R /ID [<{file_id}><{file_id}>] >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    )
    .map_err(|error| ExportError::Render(error.to_string()))?;
    Ok(pdf)
}

fn stream_object(dictionary: String, bytes: &[u8]) -> Vec<u8> {
    let mut object = dictionary.into_bytes();
    object.extend_from_slice(b"\nstream\n");
    object.extend_from_slice(bytes);
    object.extend_from_slice(b"\nendstream");
    object
}

fn require_text(payload: &Value, key: &str, expected: Option<&str>) -> Result<(), ExportError> {
    let value = payload
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ExportError::InvalidReport(format!("{key} is required")))?;
    if expected.is_some_and(|expected| expected != value) {
        return Err(ExportError::InvalidReport(format!(
            "unsupported {key}: {value}"
        )));
    }
    Ok(())
}

fn require_finite_number(payload: &Value, key: &str) -> Result<(), ExportError> {
    payload
        .get(key)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .map(|_| ())
        .ok_or_else(|| ExportError::InvalidReport(format!("{key} must be finite")))
}

fn number_array(payload: &Value, key: &str) -> Result<Vec<u64>, ExportError> {
    payload
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| ExportError::InvalidReport(format!("{key} must be an array")))?
        .iter()
        .map(|value| {
            value
                .as_u64()
                .ok_or_else(|| ExportError::InvalidReport(format!("{key} must contain integers")))
        })
        .collect()
}

fn number_matrix(
    payload: &Value,
    key: &str,
    rows: usize,
    columns: usize,
) -> Result<Vec<Vec<f64>>, ExportError> {
    let matrix = payload
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| ExportError::InvalidReport(format!("{key} must be a matrix")))?;
    if matrix.len() != rows {
        return Err(ExportError::InvalidReport(format!(
            "{key} row count does not match seeds"
        )));
    }
    matrix
        .iter()
        .map(|row| {
            let row = row
                .as_array()
                .ok_or_else(|| ExportError::InvalidReport(format!("{key} rows must be arrays")))?;
            if row.len() != columns {
                return Err(ExportError::InvalidReport(format!(
                    "{key} column count does not match horizons"
                )));
            }
            row.iter()
                .map(|value| {
                    value
                        .as_f64()
                        .filter(|value| value.is_finite() && *value >= 0.0)
                        .ok_or_else(|| {
                            ExportError::InvalidReport(format!(
                                "{key} values must be finite and non-negative"
                            ))
                        })
                })
                .collect()
        })
        .collect()
}

fn text(payload: &Value, key: &str) -> String {
    payload
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or("UNKNOWN")
        .to_owned()
}

fn number(payload: &Value, key: &str) -> f64 {
    payload.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

fn integer_text(payload: &Value, key: &str) -> String {
    payload
        .get(key)
        .and_then(Value::as_u64)
        .map(|value| value.to_string())
        .unwrap_or_else(|| "UNKNOWN".into())
}

fn format_scientific(value: f64) -> String {
    format!("{value:.4e}")
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

struct Fonts {
    inter: Font,
    mono: Font,
}

impl Fonts {
    fn load() -> Result<Self, ExportError> {
        let settings = FontSettings::default();
        let inter = Font::from_bytes(
            include_bytes!("../assets/Inter-Regular.ttf") as &[u8],
            settings.clone(),
        )
        .map_err(|error| ExportError::Render(error.to_string()))?;
        let mono = Font::from_bytes(
            include_bytes!("../assets/JetBrainsMono-Regular.ttf") as &[u8],
            settings,
        )
        .map_err(|error| ExportError::Render(error.to_string()))?;
        Ok(Self { inter, mono })
    }
}

struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Canvas {
    fn new(width: u32, height: u32, color: [u8; 3]) -> Self {
        let mut pixels = vec![0; width as usize * height as usize * 3];
        for pixel in pixels.chunks_exact_mut(3) {
            pixel.copy_from_slice(&color);
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 3], thickness: i32) {
        if y0 == y1 {
            self.fill_rect(x0, y0, x1 - x0, thickness, color);
        } else if x0 == x1 {
            self.fill_rect(x0, y0, thickness, y1 - y0, color);
        }
    }

    fn stroke_rect(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        color: [u8; 3],
        thickness: i32,
    ) {
        self.fill_rect(x, y, width, thickness, color);
        self.fill_rect(x, y + height - thickness, width, thickness, color);
        self.fill_rect(x, y, thickness, height, color);
        self.fill_rect(x + width - thickness, y, thickness, height, color);
    }

    fn fill_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: [u8; 3]) {
        let x0 = x.max(0).min(self.width as i32);
        let y0 = y.max(0).min(self.height as i32);
        let x1 = (x + width).max(0).min(self.width as i32);
        let y1 = (y + height).max(0).min(self.height as i32);
        for row in y0..y1 {
            for column in x0..x1 {
                let index = (row as usize * self.width as usize + column as usize) * 3;
                self.pixels[index..index + 3].copy_from_slice(&color);
            }
        }
    }

    fn text(&mut self, font: &Font, value: &str, size: f32, x: i32, y: i32, color: [u8; 3]) {
        let mut pen_x = x as f32;
        let baseline = y as f32 + size;
        let mut previous = None;
        for character in value.chars() {
            if let Some(previous) = previous {
                pen_x += font
                    .horizontal_kern(previous, character, size)
                    .unwrap_or(0.0);
            }
            let (metrics, bitmap) = font.rasterize(character, size);
            let glyph_x = pen_x.round() as i32 + metrics.xmin;
            let glyph_y = baseline.round() as i32 - metrics.height as i32 - metrics.ymin;
            self.blend_mask(
                glyph_x,
                glyph_y,
                metrics.width,
                metrics.height,
                &bitmap,
                color,
            );
            pen_x += metrics.advance_width;
            previous = Some(character);
        }
    }

    fn wrapped_text(
        &mut self,
        font: &Font,
        value: &str,
        size: f32,
        x: i32,
        y: i32,
        max_width: i32,
        color: [u8; 3],
        line_height: i32,
    ) -> usize {
        let lines = wrap_lines(font, value, size, max_width as f32);
        for (index, line) in lines.iter().enumerate() {
            self.text(font, line, size, x, y + index as i32 * line_height, color);
        }
        lines.len().max(1)
    }

    fn label_value(&mut self, fonts: &Fonts, x: i32, y: i32, label: &str, value: &str, width: i32) {
        self.text(&fonts.mono, label, 14.0, x, y, MUTED);
        let line = truncate_to_width(&fonts.mono, value, 18.0, width as f32);
        self.text(&fonts.mono, &line, 18.0, x, y + 28, INK);
    }

    fn blend_mask(
        &mut self,
        x: i32,
        y: i32,
        width: usize,
        height: usize,
        mask: &[u8],
        color: [u8; 3],
    ) {
        for row in 0..height {
            let target_y = y + row as i32;
            if !(0..self.height as i32).contains(&target_y) {
                continue;
            }
            for column in 0..width {
                let target_x = x + column as i32;
                if !(0..self.width as i32).contains(&target_x) {
                    continue;
                }
                let alpha = mask[row * width + column] as u16;
                if alpha == 0 {
                    continue;
                }
                let index = (target_y as usize * self.width as usize + target_x as usize) * 3;
                for channel in 0..3 {
                    let background = self.pixels[index + channel] as u16;
                    let foreground = color[channel] as u16;
                    self.pixels[index + channel] =
                        ((foreground * alpha + background * (255 - alpha)) / 255) as u8;
                }
            }
        }
    }
}

fn measure_text(font: &Font, value: &str, size: f32) -> f32 {
    let mut width = 0.0;
    let mut previous = None;
    for character in value.chars() {
        if let Some(previous) = previous {
            width += font
                .horizontal_kern(previous, character, size)
                .unwrap_or(0.0);
        }
        width += font.metrics(character, size).advance_width;
        previous = Some(character);
    }
    width
}

fn wrap_lines(font: &Font, value: &str, size: f32, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in value.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_owned()
        } else {
            format!("{current} {word}")
        };
        if !current.is_empty() && measure_text(font, &candidate, size) > max_width {
            lines.push(current);
            current = word.to_owned();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn truncate_to_width(font: &Font, value: &str, size: f32, max_width: f32) -> String {
    if measure_text(font, value, size) <= max_width {
        return value.to_owned();
    }
    let mut truncated = value.to_owned();
    while !truncated.is_empty() && measure_text(font, &format!("{truncated}..."), size) > max_width
    {
        truncated.pop();
    }
    format!("{truncated}...")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> String {
        let mut payload: Value =
            serde_json::from_str(include_str!("../tests/fixtures/benchmark_report_card.json"))
                .unwrap();
        let canonical = serde_json::to_vec(&payload).unwrap();
        payload.as_object_mut().unwrap().insert(
            "integrity_sha256".into(),
            Value::String(sha256_hex(&canonical)),
        );
        let mut json = serde_json::to_string_pretty(&payload).unwrap();
        json.push('\n');
        json
    }

    #[test]
    fn canonical_report_hash_round_trips_and_rejects_mutation() {
        let report = fixture();
        let expected = verify_canonical_report(&report).unwrap();
        assert!(is_sha256(&expected));

        let mutated = report.replace("\"global_rel_l2\": 0.15", "\"global_rel_l2\": 0.16");
        assert!(matches!(
            verify_canonical_report(&mutated),
            Err(ExportError::Integrity(_))
        ));
    }

    #[test]
    fn png_and_pdf_are_deterministic_golden_artifacts() {
        let report = fixture();
        for (format, expected) in [
            (
                ExportFormat::Png,
                include_str!("../tests/fixtures/benchmark_report_card.png.sha256").trim(),
            ),
            (
                ExportFormat::Pdf,
                include_str!("../tests/fixtures/benchmark_report_card.pdf.sha256").trim(),
            ),
        ] {
            let first = export_report(&report, format).unwrap();
            let second = export_report(&report, format).unwrap();
            assert_eq!(first.bytes, second.bytes);
            assert_eq!(first.content_sha256, expected);
            assert_eq!(first.media_type(), format.media_type());
            first.verify().unwrap();
        }
    }

    #[test]
    fn exported_formats_round_trip_structure_and_metadata() {
        let report = fixture();
        let png = export_report(&report, ExportFormat::Png).unwrap();
        let decoder = png::Decoder::new(std::io::Cursor::new(&png.bytes));
        let reader = decoder.read_info().unwrap();
        assert_eq!(reader.info().width, PNG_WIDTH);
        assert!(reader.info().height >= MIN_PNG_HEIGHT);
        assert_eq!(reader.info().color_type, png::ColorType::Rgb);
        assert!(reader
            .info()
            .uncompressed_latin1_text
            .iter()
            .any(|chunk| chunk.keyword == "ReynAuthenticityStatus" && chunk.text == "UNSIGNED"));

        let pdf = export_report(&report, ExportFormat::Pdf).unwrap();
        assert!(pdf.bytes.starts_with(b"%PDF-1.4\n"));
        assert!(pdf.bytes.ends_with(b"%%EOF\n"));
        assert!(contains_bytes(&pdf.bytes, b"/Subtype /Image"));
        assert!(contains_bytes(&pdf.bytes, b"Authenticity: UNSIGNED"));
    }

    #[test]
    fn malformed_matrices_sources_and_fabricated_signatures_are_rejected() {
        let report = fixture();
        let mut value: Value = serde_json::from_str(&report).unwrap();
        value.as_object_mut().unwrap().remove("integrity_sha256");
        value["rel_l2"] = serde_json::json!([[0.1]]);
        let canonical = serde_json::to_vec(&value).unwrap();
        value["integrity_sha256"] = Value::String(sha256_hex(&canonical));
        assert!(matches!(
            verify_canonical_report(&serde_json::to_string(&value).unwrap()),
            Err(ExportError::InvalidReport(_))
        ));

        let mut unsigned: Value = serde_json::from_str(&report).unwrap();
        unsigned.as_object_mut().unwrap().remove("integrity_sha256");
        unsigned["authenticity"] = serde_json::json!({
            "status": "SIGNED",
            "signature": "not-a-signature"
        });
        let canonical = serde_json::to_vec(&unsigned).unwrap();
        unsigned["integrity_sha256"] = Value::String(sha256_hex(&canonical));
        assert!(matches!(
            verify_canonical_report(&serde_json::to_string(&unsigned).unwrap()),
            Err(ExportError::InvalidReport(_))
        ));

        let mut bad_source: Value = serde_json::from_str(&report).unwrap();
        bad_source
            .as_object_mut()
            .unwrap()
            .remove("integrity_sha256");
        bad_source["selected_cell_evidence"]["spatial_variable_evidence"]["variables"][0]
            ["sources"]["model"] = Value::String("PHYSICAL_TRUTH".into());
        let canonical = serde_json::to_vec(&bad_source).unwrap();
        bad_source["integrity_sha256"] = Value::String(sha256_hex(&canonical));
        assert!(matches!(
            verify_canonical_report(&serde_json::to_string(&bad_source).unwrap()),
            Err(ExportError::InvalidReport(_))
        ));
    }

    #[test]
    fn artifact_verifier_detects_byte_mutation() {
        let report = fixture();
        let mut artifact = export_report(&report, ExportFormat::Png).unwrap();
        let last = artifact.bytes.len() - 1;
        artifact.bytes[last] ^= 1;
        assert!(matches!(artifact.verify(), Err(ExportError::Integrity(_))));
    }

    #[test]
    fn maximum_interactive_suite_and_long_warnings_fit_both_formats() {
        let mut value: Value = serde_json::from_str(&fixture()).unwrap();
        value.as_object_mut().unwrap().remove("integrity_sha256");
        value["seeds"] = serde_json::json!([70000, 70001, 70002, 70003, 70004, 70005]);
        value["horizons"] = serde_json::json!([1, 4, 8, 16]);
        value["rel_l2"] = serde_json::json!([
            [0.1, 0.2, 0.3, 0.4],
            [0.11, 0.21, 0.31, 0.41],
            [0.12, 0.22, 0.32, 0.42],
            [0.13, 0.23, 0.33, 0.43],
            [0.14, 0.24, 0.34, 0.44],
            [0.15, 0.25, 0.35, 0.45]
        ]);
        value["persistence"] = serde_json::json!([
            [0.5, 0.6, 0.7, 0.8],
            [0.51, 0.61, 0.71, 0.81],
            [0.52, 0.62, 0.72, 0.82],
            [0.53, 0.63, 0.73, 0.83],
            [0.54, 0.64, 0.74, 0.84],
            [0.55, 0.65, 0.75, 0.85]
        ]);
        value["warnings"] = Value::Array(
            (0..12)
                .map(|index| {
                    Value::String(format!(
                        "warning {index}: this intentionally long diagnostic remains visible in the exported scientific evidence rather than being silently clipped"
                    ))
                })
                .collect(),
        );
        let canonical = serde_json::to_vec(&value).unwrap();
        value["integrity_sha256"] = Value::String(sha256_hex(&canonical));
        let report = serde_json::to_string(&value).unwrap();

        for format in [ExportFormat::Png, ExportFormat::Pdf] {
            export_report(&report, format).unwrap().verify().unwrap();
        }
    }

    #[test]
    fn detached_signature_is_consistent_across_json_png_and_pdf() {
        let report = fixture();
        let canonical_payload_sha256 = verify_canonical_report(&report).unwrap();
        let value: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(value["authenticity"]["status"], "UNSIGNED");
        let provider = crate::signing::DeterministicTestProvider::new("export-vector");
        let key = provider.public_key_record();
        let signature = crate::signing::sign_canonical_payload(
            &provider,
            &key,
            false,
            &crate::signing::SigningLineage {
                run_id: value["run_id"].as_str().unwrap().into(),
                report_schema: value["report_schema"].as_str().unwrap().into(),
                canonical_report_sha256: crate::signing::sha256_hex(report.as_bytes()),
                canonical_payload_sha256: canonical_payload_sha256.clone(),
                created_utc_unix: value["timestamp_unix"].as_u64().unwrap(),
            },
        )
        .unwrap();
        let sidecar_sha256 = signature.content_sha256().unwrap();
        let mut rendered = Vec::new();
        for format in [ExportFormat::Png, ExportFormat::Pdf] {
            let first = export_report_with_signature(&report, format, &signature).unwrap();
            let second = export_report_with_signature(&report, format, &signature).unwrap();
            assert_eq!(first.bytes, second.bytes);
            assert_eq!(first.canonical_payload_sha256, canonical_payload_sha256);
            assert_eq!(
                first.signature_sidecar_sha256.as_deref(),
                Some(sidecar_sha256.as_str())
            );
            first.verify().unwrap();
            rendered.push(first);
        }
        assert!(contains_bytes(
            &rendered[0].bytes,
            key.key_fingerprint_sha256.as_bytes()
        ));
        assert!(contains_bytes(
            &rendered[1].bytes,
            key.key_fingerprint_sha256.as_bytes()
        ));
        assert_eq!(
            verify_report_signature(
                &report,
                &signature,
                &VerificationPolicy::new(
                    [key.key_fingerprint_sha256],
                    std::iter::empty::<String>(),
                ),
            )
            .status,
            VerificationStatus::VerifiedTrustedKey
        );
    }

    #[test]
    fn canonical_report_mutation_invalidates_detached_signature() {
        let report = fixture();
        let canonical_payload_sha256 = verify_canonical_report(&report).unwrap();
        let value: Value = serde_json::from_str(&report).unwrap();
        let provider = crate::signing::DeterministicTestProvider::new("mutation-vector");
        let key = provider.public_key_record();
        let signature = crate::signing::sign_canonical_payload(
            &provider,
            &key,
            false,
            &crate::signing::SigningLineage {
                run_id: value["run_id"].as_str().unwrap().into(),
                report_schema: value["report_schema"].as_str().unwrap().into(),
                canonical_report_sha256: crate::signing::sha256_hex(report.as_bytes()),
                canonical_payload_sha256,
                created_utc_unix: 42,
            },
        )
        .unwrap();
        let mutated = report.replace("\"global_rel_l2\": 0.15", "\"global_rel_l2\": 0.16");
        assert_eq!(
            verify_report_signature(
                &mutated,
                &signature,
                &VerificationPolicy::portable_untrusted(),
            )
            .status,
            VerificationStatus::HashMismatch
        );
        assert!(export_report_with_signature(&mutated, ExportFormat::Png, &signature).is_err());
    }
}
