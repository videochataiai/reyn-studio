//! Case-centered engineering workflow state.
//!
//! This module deliberately separates supported external-flow analysis from
//! developer demonstrations. It contains no renderer or engine assumptions, so
//! project persistence and scientific gates remain testable without a GPU.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::Write;

pub const EXTERNAL_FLOW_CONTRACT: &str = "external_fixed_body.v1";
pub const INTERNAL_FLOW_CONTRACT: &str = "internal_flow.reference_only.v1";
pub const SURFACE_LOAD_METHOD: &str = "diffuse_interface_traction.v1";
pub const ENGINEERING_RESULT_SCHEMA: &str = "engineering_result.v1";
pub const ENGINEERING_FIELD_SCHEMA: &str = "engineering_field.f32le.v1";
pub const FEA_LOAD_SCHEMA: &str = "reyn_fea_surface_loads.v1";

/// The frame every reported force and moment coefficient is expressed in.
///
/// The solver's free stream is fixed on `+X` and body orientation is applied by
/// rotating the geometry, so the coefficient axes are **wind axes**: `Cd` is
/// always streamwise drag, `Cl` always vertical lift, no matter how the body is
/// pitched. Converting to body axes would require rotating the reported vectors
/// by the negative body orientation; Reyn does not do that silently.
pub const COEFFICIENT_REFERENCE_FRAME: &str =
    "wind axes · drag +X (stream), side +Y, lift +Z — not rotated with the body";

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaseStage {
    #[default]
    Source,
    Preflight,
    Setup,
    Ready,
    Running,
    Results,
    Evidence,
}

impl CaseStage {
    pub fn progress_index(self) -> usize {
        match self {
            Self::Source => 0,
            Self::Preflight => 1,
            Self::Setup => 2,
            Self::Ready | Self::Running => 3,
            Self::Results => 4,
            Self::Evidence => 5,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LengthUnit {
    #[default]
    Unknown,
    Millimeter,
    Centimeter,
    Meter,
    Inch,
    Foot,
}

impl LengthUnit {
    pub const ALL: [Self; 6] = [
        Self::Unknown,
        Self::Millimeter,
        Self::Centimeter,
        Self::Meter,
        Self::Inch,
        Self::Foot,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "Select units…",
            Self::Millimeter => "Millimeters",
            Self::Centimeter => "Centimeters",
            Self::Meter => "Meters",
            Self::Inch => "Inches",
            Self::Foot => "Feet",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Millimeter => "mm",
            Self::Centimeter => "cm",
            Self::Meter => "m",
            Self::Inch => "in",
            Self::Foot => "ft",
        }
    }

    pub fn meters_per_unit(self) -> Option<f64> {
        match self {
            Self::Unknown => None,
            Self::Millimeter => Some(1e-3),
            Self::Centimeter => Some(1e-2),
            Self::Meter => Some(1.0),
            Self::Inch => Some(0.0254),
            Self::Foot => Some(0.3048),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct OperatingPoint {
    pub length_unit: LengthUnit,
    pub reference_length: f64,
    pub velocity: f64,
    pub density: f64,
    pub viscosity: f64,
    pub reference_pressure: f64,
    pub flow_direction: [f64; 3],
    pub horizon_steps: u32,
}

impl Default for OperatingPoint {
    fn default() -> Self {
        Self {
            length_unit: LengthUnit::Unknown,
            reference_length: 1.0,
            velocity: 1.0,
            density: 1.225,
            viscosity: 1.81e-5,
            reference_pressure: 101_325.0,
            flow_direction: [1.0, 0.0, 0.0],
            horizon_steps: 4,
        }
    }
}

impl OperatingPoint {
    pub fn reynolds(&self) -> Option<f64> {
        let scale = self.length_unit.meters_per_unit()?;
        if self.velocity <= 0.0
            || self.reference_length <= 0.0
            || self.density <= 0.0
            || self.viscosity <= 0.0
        {
            return None;
        }
        Some(self.density * self.velocity * self.reference_length * scale / self.viscosity)
    }

    pub fn dynamic_pressure(&self) -> Option<f64> {
        (self.density > 0.0 && self.velocity > 0.0)
            .then_some(0.5 * self.density * self.velocity * self.velocity)
    }

    pub fn validation(&self, max_steps: u32) -> Vec<String> {
        let mut issues = Vec::new();
        if self.length_unit == LengthUnit::Unknown {
            issues.push("Geometry units must be confirmed.".into());
        }
        if !self.reference_length.is_finite() || self.reference_length <= 0.0 {
            issues.push("Reference length must be positive.".into());
        }
        if !self.velocity.is_finite() || self.velocity <= 0.0 {
            issues.push("Free-stream speed must be positive.".into());
        }
        if !self.density.is_finite() || self.density <= 0.0 {
            issues.push("Density must be positive.".into());
        }
        if !self.viscosity.is_finite() || self.viscosity <= 0.0 {
            issues.push("Dynamic viscosity must be positive.".into());
        }
        if !self.reference_pressure.is_finite() {
            issues.push("Reference pressure must be finite.".into());
        }
        if self
            .flow_direction
            .iter()
            .any(|component| !component.is_finite())
        {
            issues.push("Flow direction must contain finite components.".into());
        }
        let norm = self
            .flow_direction
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt();
        if norm <= 1e-12 {
            issues.push("Flow direction must be nonzero.".into());
        } else {
            let normalized = self.flow_direction.map(|component| component / norm);
            if (normalized[0] - 1.0).abs() > 1e-9
                || normalized[1].abs() > 1e-9
                || normalized[2].abs() > 1e-9
            {
                issues.push("The current fixed-body contract supports +X free-stream only.".into());
            }
        }
        if max_steps > 0 && (self.horizon_steps == 0 || self.horizon_steps > max_steps) {
            issues.push(format!(
                "Horizon must lie inside the selected model support (1–{max_steps})."
            ));
        }
        if let Some(reynolds) = self.reynolds() {
            if !(60.0..=400.0).contains(&reynolds) {
                issues.push(format!(
                    "Reynolds number {reynolds:.1} lies outside the qualified 60–400 envelope."
                ));
            }
        }
        issues
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupportIssue {
    pub code: &'static str,
    pub message: String,
    pub waivable: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct GeometryPreflight {
    pub source_sha256: String,
    pub source_bytes: u64,
    pub triangles: usize,
    pub components: usize,
    pub degenerate_triangles: usize,
    pub boundary_edges: usize,
    pub non_manifold_edges: usize,
    pub inconsistent_winding_edges: usize,
    pub self_intersection_pairs: usize,
    /// Source-space signed volume from triangle winding. This is provenance,
    /// not a load-normal input: diffuse-interface loads derive their normal
    /// from the approved occupancy mask.
    pub source_signed_volume: f64,
    pub source_extents: [f64; 3],
    pub proposed_scale: f64,
    pub solver_characteristic_length: f64,
    /// Body orientation applied to the source geometry before voxelization, in
    /// degrees: angle of attack about `+Y`, yaw about `+Z`, roll about `+X`.
    /// The free stream cannot be rotated, so the body is; these angles are
    /// folded into `transform_4x4` and travel with the case contract.
    pub angle_of_attack_deg: f64,
    pub yaw_deg: f64,
    pub roll_deg: f64,
    pub transform_4x4: [f64; 16],
    pub target_grid: usize,
    pub solid_voxels: usize,
    pub voxel_components: usize,
    pub minimum_cells_across: usize,
    pub boundary_clearance_cells: usize,
    /// Three-axis occupancy disagreement, normalized by the union of cells
    /// classified solid by at least one axis.
    pub voxel_axis_disagreement_fraction: f64,
    pub voxel_odd_crossing_rows: [usize; 3],
    pub voxel_classification_version: u32,
    pub warnings: Vec<String>,
    pub waivers: Vec<String>,
    pub transform_approved: bool,
}

impl GeometryPreflight {
    /// Above two percent, independent ray directions disagree by materially
    /// more than the ~one-percent curved-surface discretization floor measured
    /// on the valid sphere fixture. Geometry classification is then blocked.
    pub const MAX_AXIS_DISAGREEMENT_FRACTION: f64 = 0.02;

    /// Angle of attack, yaw, and roll in degrees, in the order
    /// `cad::BodyOrientation::from_degrees` expects.
    pub fn body_orientation_degrees(&self) -> [f64; 3] {
        [self.angle_of_attack_deg, self.yaw_deg, self.roll_deg]
    }

    pub fn body_is_aligned(&self) -> bool {
        self.body_orientation_degrees()
            .iter()
            .all(|angle| angle.abs() < 1e-9)
    }

    /// Human-readable orientation for chips, evidence rows, and reports.
    pub fn body_orientation_summary(&self) -> String {
        if self.body_is_aligned() {
            "as imported · aligned with the +X stream".to_owned()
        } else {
            format!(
                "α {:+.2}° · β {:+.2}° · roll {:+.2}° (body rotated, stream fixed +X)",
                self.angle_of_attack_deg, self.yaw_deg, self.roll_deg
            )
        }
    }

    pub fn support_issues(&self) -> Vec<SupportIssue> {
        let mut issues = Vec::new();
        let mut issue = |code, message: String, waivable| {
            issues.push(SupportIssue {
                code,
                message,
                waivable,
            });
        };
        if self.source_sha256.len() != 64
            || !self
                .source_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            issue(
                "source.invalid_sha256",
                "The STL source has no canonical SHA-256 identity.".into(),
                false,
            );
        }
        if self.source_bytes == 0 {
            issue(
                "source.empty",
                "The STL source contains no bytes.".into(),
                false,
            );
        }
        if self.triangles == 0 {
            issue(
                "mesh.no_triangles",
                "No triangles were parsed.".into(),
                false,
            );
        }
        if self.degenerate_triangles > 0 {
            issue(
                "mesh.degenerate_triangles",
                format!(
                    "{} degenerate triangles require repair or a named waiver.",
                    self.degenerate_triangles
                ),
                true,
            );
        }
        if self.boundary_edges > 0 {
            issue(
                "mesh.open_boundary",
                format!(
                    "{} open boundary edges make inside/outside classification and surface loads untrustworthy.",
                    self.boundary_edges
                ),
                false,
            );
        }
        if self.non_manifold_edges > 0 {
            issue(
                "mesh.non_manifold",
                format!(
                    "{} non-manifold edges require repair.",
                    self.non_manifold_edges
                ),
                false,
            );
        }
        if self.inconsistent_winding_edges > 0 {
            issue(
                "mesh.inconsistent_winding",
                format!(
                    "{} shared edges have inconsistent triangle winding.",
                    self.inconsistent_winding_edges
                ),
                false,
            );
        }
        if self.self_intersection_pairs > 0 {
            issue(
                "mesh.self_intersection",
                format!(
                    "{} non-adjacent triangle pairs intersect; the solid interior is ambiguous.",
                    self.self_intersection_pairs
                ),
                false,
            );
        }
        if self.components > 1 {
            issue(
                "mesh.disconnected_components",
                format!(
                    "The STL contains {} surface components; nested shells, intersections, and multiple bodies are ambiguous in the current solid contract.",
                    self.components
                ),
                false,
            );
        }
        if self
            .source_extents
            .iter()
            .any(|extent| !extent.is_finite() || *extent <= 0.0)
        {
            issue(
                "mesh.invalid_extents",
                "All source extents must be finite and positive.".into(),
                false,
            );
        }
        if !self.proposed_scale.is_finite() || self.proposed_scale <= 0.0 {
            issue(
                "transform.invalid_scale",
                "The proposed preprocessing scale must be finite and positive.".into(),
                false,
            );
        }
        if !self.solver_characteristic_length.is_finite()
            || self.solver_characteristic_length <= 0.0
        {
            issue(
                "transform.invalid_characteristic_length",
                "The solver characteristic length must be finite and positive.".into(),
                false,
            );
        }
        if self
            .transform_4x4
            .iter()
            .any(|component| !component.is_finite())
        {
            issue(
                "transform.non_finite",
                "The preprocessing transform contains a non-finite value.".into(),
                false,
            );
        }
        if self
            .body_orientation_degrees()
            .iter()
            .any(|angle| !angle.is_finite() || angle.abs() > 180.0)
        {
            issue(
                "transform.invalid_body_orientation",
                "Body orientation angles must be finite and within ±180°.".into(),
                false,
            );
        }
        if self.target_grid == 0 {
            issue(
                "voxel.invalid_grid",
                "The target voxel grid is unavailable.".into(),
                false,
            );
        }
        if self.solid_voxels == 0 {
            issue(
                "voxel.empty",
                "Voxelization produced an empty solid.".into(),
                false,
            );
        }
        if self.voxel_classification_version < 2 {
            issue(
                "voxel.classification_stale",
                "This case predates three-axis occupancy validation and must be re-imported."
                    .into(),
                false,
            );
        }
        if !self.voxel_axis_disagreement_fraction.is_finite()
            || self.voxel_axis_disagreement_fraction > Self::MAX_AXIS_DISAGREEMENT_FRACTION
        {
            issue(
                "voxel.axis_disagreement",
                format!(
                    "Independent axis classifications disagree on {:.2}% of candidate solid cells; the hard limit is {:.2}%.",
                    self.voxel_axis_disagreement_fraction * 100.0,
                    Self::MAX_AXIS_DISAGREEMENT_FRACTION * 100.0
                ),
                false,
            );
        }
        if self.voxel_components > 1 {
            issue(
                "voxel.disconnected_components",
                format!(
                    "Voxelization contains {} disconnected components.",
                    self.voxel_components
                ),
                true,
            );
        }
        if self.minimum_cells_across < 3 {
            issue(
                "voxel.under_resolved",
                format!(
                    "Critical thickness resolves to only {} cells; at least 3 are required.",
                    self.minimum_cells_across
                ),
                true,
            );
        }
        if self.boundary_clearance_cells < 2 {
            issue(
                "voxel.boundary_clearance",
                "The solid is too close to a solver boundary.".into(),
                true,
            );
        }
        if !self.transform_approved {
            issue(
                "transform.approval_required",
                "The units and preprocessing transform require approval.".into(),
                false,
            );
        }
        issues
    }

    fn waiver_covers(&self, issue: &SupportIssue) -> bool {
        issue.waivable
            && self.waivers.iter().any(|waiver| {
                waiver == &issue.message
                    || waiver
                        .strip_prefix(issue.code)
                        .is_some_and(|detail| detail.starts_with(':'))
            })
    }

    pub fn blocking_issues(&self) -> Vec<String> {
        self.support_issues()
            .into_iter()
            .filter(|issue| !self.waiver_covers(issue))
            .map(|issue| issue.message)
            .collect()
    }

    pub fn record_waiver(&mut self, code: &str, rationale: &str) -> Result<(), String> {
        let rationale = rationale.trim();
        if rationale.len() < 8 {
            return Err("A named waiver requires a specific rationale.".into());
        }
        let issue = self
            .support_issues()
            .into_iter()
            .find(|issue| issue.code == code)
            .ok_or_else(|| format!("No active preflight issue has code {code}."))?;
        if matches!(
            code,
            "mesh.open_boundary"
                | "mesh.non_manifold"
                | "mesh.inconsistent_winding"
                | "mesh.self_intersection"
                | "mesh.disconnected_components"
                | "voxel.axis_disagreement"
                | "voxel.classification_stale"
        ) {
            return Err(
                "Measured geometry-fidelity gates cannot be replaced by a prose waiver.".into(),
            );
        }
        if !issue.waivable {
            return Err(format!("{} cannot be waived.", issue.message));
        }
        let waiver = format!("{}: {}", issue.code, rationale);
        if !self.waivers.contains(&waiver) {
            self.waivers.push(waiver);
            self.waivers.sort();
        }
        Ok(())
    }

    pub fn ready(&self) -> bool {
        self.blocking_issues().is_empty()
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct ModelSupport {
    pub status: String,
    pub dimension: u32,
    pub grid: u32,
    pub input_channels: u32,
    pub output_channels: u32,
    pub scenario: String,
    pub physics_contract: String,
}

impl ModelSupport {
    pub fn validation(&self, target_grid: usize) -> Vec<String> {
        let mut issues = Vec::new();
        if self.status == "unavailable" {
            issues.push(
                "No compatible verified 3D .reynmodel bundle is available; geometry review is available, but inference is blocked."
                    .into(),
            );
        } else if self.status == "invalid" || self.status.trim().is_empty() {
            issues.push("The selected checkpoint has no accepted validation state.".into());
        }
        if self.dimension != 3 {
            issues.push("External STL execution requires a 3D checkpoint.".into());
        }
        if self.grid as usize != target_grid {
            issues.push(format!(
                "The checkpoint grid {}³ does not match preprocessing grid {}³.",
                self.grid, target_grid
            ));
        }
        if self.input_channels <= self.output_channels || self.output_channels != 3 {
            issues.push(format!(
                "The checkpoint channel contract {}→{} is not geometry-conditioned 3D velocity.",
                self.input_channels, self.output_channels
            ));
        }
        if self.scenario != "obstacle" {
            issues.push(format!(
                "The checkpoint scenario {:?} is not the supported fixed-body obstacle regime.",
                self.scenario
            ));
        }
        issues
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct EngineeringResult {
    pub method: String,
    pub cp_min: f64,
    pub cp_max: f64,
    pub force_coefficients: [f64; 3],
    pub moment_coefficients: [f64; 3],
    pub force_newtons: [f64; 3],
    pub moment_newton_meters: [f64; 3],
    pub surface_area_m2: f64,
    pub pressure_force_fraction: f64,
    pub load_hotspot: [f64; 3],
    pub suction_hotspot: [f64; 3],
    pub divergence_rms: f64,
    pub wake_deficit_peak: f64,
    pub wake_deficit_mean: f64,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ExternalFlowCase {
    pub stage: CaseStage,
    pub case_id: String,
    pub name: String,
    pub source_name: String,
    pub source_revision_id: Option<String>,
    pub case_revision_id: Option<String>,
    pub model_id: String,
    pub model_sha256: Option<String>,
    pub model_max_steps: u32,
    pub model_support: ModelSupport,
    pub preflight: GeometryPreflight,
    pub operating: OperatingPoint,
    pub result: Option<EngineeringResult>,
    pub parent_run_id: Option<String>,
}

/// Scope key for one in-memory case-draft history.
///
/// Identity belongs to the scope, not to individual snapshots. Changing the
/// project, case, or source therefore rebases the stack instead of making an
/// identity transition undoable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseDraftScope {
    project_id: String,
    case_id: String,
    source_revision_id: Option<String>,
    source_sha256: String,
}

impl CaseDraftScope {
    pub fn new(
        project_id: impl Into<String>,
        case_id: impl Into<String>,
        source_revision_id: Option<String>,
        source_sha256: impl Into<String>,
    ) -> Self {
        Self {
            project_id: project_id.into(),
            case_id: case_id.into(),
            source_revision_id,
            source_sha256: source_sha256.into(),
        }
    }
}

/// Reversible inputs from an uncommitted external-flow case draft.
///
/// Deliberately absent: source/model identities and hashes, geometry bytes and
/// transforms, case/run/evidence IDs, results, signing state, and lineage.
/// Those are immutable evidence or transition boundaries, never undo payload.
#[derive(Clone, Debug, PartialEq)]
pub struct CaseDraftSnapshot {
    operating: OperatingPoint,
    transform_approved: bool,
    waivers: Vec<String>,
}

impl CaseDraftSnapshot {
    pub fn capture(case: &ExternalFlowCase) -> Self {
        Self {
            operating: case.operating.clone(),
            transform_approved: case.preflight.transform_approved,
            waivers: case.preflight.waivers.clone(),
        }
    }

    pub fn restore(&self, case: &mut ExternalFlowCase) {
        case.operating = self.operating.clone();
        case.preflight.transform_approved = self.transform_approved;
        case.preflight.waivers = self.waivers.clone();
    }
}

/// Maximum number of reversible draft transactions retained per active case.
pub const CASE_DRAFT_HISTORY_LIMIT: usize = 64;

/// Bounded, session-only undo/redo for safe case-draft inputs.
#[derive(Clone, Debug)]
pub struct CaseDraftHistory {
    scope: Option<CaseDraftScope>,
    undo: VecDeque<CaseDraftSnapshot>,
    redo: VecDeque<CaseDraftSnapshot>,
    limit: usize,
}

impl Default for CaseDraftHistory {
    fn default() -> Self {
        Self::with_limit(CASE_DRAFT_HISTORY_LIMIT)
    }
}

impl CaseDraftHistory {
    pub fn with_limit(limit: usize) -> Self {
        Self {
            scope: None,
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            limit: limit.max(1),
        }
    }

    pub fn clear(&mut self) {
        self.scope = None;
        self.undo.clear();
        self.redo.clear();
    }

    pub fn rebase(&mut self, scope: CaseDraftScope) {
        self.scope = Some(scope);
        self.undo.clear();
        self.redo.clear();
    }

    pub fn can_undo(&self, scope: &CaseDraftScope) -> bool {
        self.scope.as_ref() == Some(scope) && !self.undo.is_empty()
    }

    pub fn can_redo(&self, scope: &CaseDraftScope) -> bool {
        self.scope.as_ref() == Some(scope) && !self.redo.is_empty()
    }

    /// Record one before/after draft edit. `coalesce` means the edit is another
    /// repaint of the same active DragValue/text interaction, so its original
    /// before-state is already the top undo transaction.
    pub fn record_change(
        &mut self,
        scope: CaseDraftScope,
        before: CaseDraftSnapshot,
        after: &CaseDraftSnapshot,
        coalesce: bool,
    ) -> bool {
        self.ensure_scope(scope);
        // Any new edit, including one that returned to the same value, forks
        // history and makes the redo branch invalid.
        self.redo.clear();
        if before == *after || coalesce {
            return false;
        }
        Self::push_bounded(&mut self.undo, before, self.limit);
        true
    }

    pub fn undo(
        &mut self,
        scope: CaseDraftScope,
        current: CaseDraftSnapshot,
    ) -> Option<CaseDraftSnapshot> {
        self.ensure_scope(scope);
        let previous = self.undo.pop_back()?;
        Self::push_bounded(&mut self.redo, current, self.limit);
        Some(previous)
    }

    pub fn redo(
        &mut self,
        scope: CaseDraftScope,
        current: CaseDraftSnapshot,
    ) -> Option<CaseDraftSnapshot> {
        self.ensure_scope(scope);
        let next = self.redo.pop_back()?;
        Self::push_bounded(&mut self.undo, current, self.limit);
        Some(next)
    }

    fn ensure_scope(&mut self, scope: CaseDraftScope) {
        if self.scope.as_ref() != Some(&scope) {
            self.rebase(scope);
        }
    }

    fn push_bounded(
        stack: &mut VecDeque<CaseDraftSnapshot>,
        snapshot: CaseDraftSnapshot,
        limit: usize,
    ) {
        stack.push_back(snapshot);
        while stack.len() > limit {
            stack.pop_front();
        }
    }
}

impl ExternalFlowCase {
    pub fn readiness_issues(&self) -> Vec<String> {
        let mut issues = self.preflight.blocking_issues();
        issues.extend(self.operating.validation(self.model_max_steps));
        if self.model_id.trim().is_empty() {
            issues.push("A qualified geometry-conditioned 3D model is required.".into());
        }
        if self.model_sha256.as_deref().is_none_or(|digest| {
            digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
        }) {
            issues.push("The selected model requires a recorded checkpoint SHA-256.".into());
        }
        if self.model_max_steps == 0 {
            issues.push("The selected model does not declare a supported horizon.".into());
        }
        issues.extend(self.model_support.validation(self.preflight.target_grid));
        issues
    }

    pub fn ready(&self) -> bool {
        self.readiness_issues().is_empty()
    }

    pub fn exact_contract(&self) -> serde_json::Value {
        serde_json::json!({
            "kind": EXTERNAL_FLOW_CONTRACT,
            "case_id": self.case_id,
            "case_revision_id": self.case_revision_id,
            "source_revision_id": self.source_revision_id,
            "source_sha256": self.preflight.source_sha256,
            "model": {
                "id": self.model_id,
                "sha256": self.model_sha256,
                "max_steps": self.model_max_steps,
                "support": self.model_support,
            },
            "operating_point": self.operating,
            "preflight": self.preflight,
            "surface_load_method": SURFACE_LOAD_METHOD,
        })
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct InternalBoundaryAssignment {
    pub region_id: Option<String>,
    pub name: String,
    /// `velocity_inlet`, `mass_flow_inlet`, `pressure_outlet`, or `wall`.
    pub role: String,
    pub velocity_mps: Option<[f64; 3]>,
    pub mass_flow_kg_s: Option<f64>,
    pub static_pressure_pa: Option<f64>,
    pub temperature_k: Option<f64>,
    pub wall_roughness_m: Option<f64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct InternalFluidProperties {
    pub material_name: String,
    pub density_kg_m3: Option<f64>,
    pub dynamic_viscosity_pa_s: Option<f64>,
    pub temperature_k: Option<f64>,
    pub contaminant_diffusivity_m2_s: Option<f64>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct InternalPressureDropTarget {
    pub inlet_region_id: String,
    pub outlet_region_id: String,
    pub target_pa: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct InternalFlowTargets {
    pub pressure_drop: Vec<InternalPressureDropTarget>,
    pub mass_flow_balance_tolerance_fraction: f64,
    pub comfort_quantities: Vec<String>,
    pub contaminant_quantities: Vec<String>,
}

impl Default for InternalFlowTargets {
    fn default() -> Self {
        Self {
            pressure_drop: Vec::new(),
            mass_flow_balance_tolerance_fraction: 0.01,
            comfort_quantities: vec![
                "occupied-zone air speed".into(),
                "temperature when energy transport exists".into(),
            ],
            contaminant_quantities: vec![
                "concentration when scalar transport exists".into(),
                "age of air when supported".into(),
            ],
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct InternalReferenceStrategy {
    pub required: bool,
    pub solver_name: Option<String>,
    pub solver_version: Option<String>,
    pub configuration_sha256: Option<String>,
    pub mesh_identity: Option<String>,
    pub quantities: Vec<String>,
    pub validation_state: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct InternalFlowContract {
    pub schema_version: u32,
    pub contract_kind: String,
    pub case_family: String,
    pub intent: String,
    pub geometry_revision_id: Option<String>,
    pub inlet_assignments: Vec<InternalBoundaryAssignment>,
    pub outlet_assignments: Vec<InternalBoundaryAssignment>,
    pub wall_assignments: Vec<InternalBoundaryAssignment>,
    pub fluid_properties: InternalFluidProperties,
    pub targets: InternalFlowTargets,
    pub reference_strategy: InternalReferenceStrategy,
    pub compatible_model_id: Option<String>,
    pub execution_available: bool,
    pub required_assignments: Vec<String>,
    pub quantities_of_interest: Vec<String>,
    pub limitation: String,
}

impl Default for InternalFlowContract {
    fn default() -> Self {
        Self {
            schema_version: 1,
            contract_kind: INTERNAL_FLOW_CONTRACT.into(),
            case_family: "internal_hvac".into(),
            intent: "Reference-only contract for ducts, rooms, and HVAC flow paths.".into(),
            geometry_revision_id: None,
            inlet_assignments: Vec::new(),
            outlet_assignments: Vec::new(),
            wall_assignments: Vec::new(),
            fluid_properties: InternalFluidProperties::default(),
            targets: InternalFlowTargets::default(),
            reference_strategy: InternalReferenceStrategy {
                required: true,
                quantities: vec![
                    "pressure drop".into(),
                    "mass-flow balance".into(),
                    "flow distribution".into(),
                ],
                validation_state: "unavailable".into(),
                ..InternalReferenceStrategy::default()
            },
            compatible_model_id: None,
            execution_available: false,
            required_assignments: vec![
                "one or more named inlet regions and conditions".into(),
                "one or more named outlet regions and conditions".into(),
                "wall regions".into(),
                "fluid material properties".into(),
            ],
            quantities_of_interest: vec![
                "pressure drop".into(),
                "mass-flow balance".into(),
                "flow distribution".into(),
                "comfort or contaminant metrics when transported scalars exist".into(),
            ],
            limitation:
                "The current fixed-body external-flow model is incompatible. Internal execution remains blocked until a qualified internal solver/model and reference suite are shipped."
                    .into(),
        }
    }
}

impl InternalFlowContract {
    pub fn exact_contract(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("internal-flow contract is serializable")
    }

    pub fn execution_blockers(&self) -> Vec<String> {
        if !self.execution_available {
            return vec![self.limitation.clone()];
        }
        let mut blockers = Vec::new();
        if self.compatible_model_id.is_none() {
            blockers.push("A qualified internal-flow solver/model contract is required.".into());
        }
        if self.inlet_assignments.is_empty()
            || self.outlet_assignments.is_empty()
            || self.wall_assignments.is_empty()
        {
            blockers.push("Named inlet, outlet, and wall assignments are required.".into());
        }
        if self
            .fluid_properties
            .density_kg_m3
            .is_none_or(|value| !value.is_finite() || value <= 0.0)
            || self
                .fluid_properties
                .dynamic_viscosity_pa_s
                .is_none_or(|value| !value.is_finite() || value <= 0.0)
        {
            blockers.push("Positive density and dynamic viscosity are required.".into());
        }
        if !self.reference_strategy.required
            || self.reference_strategy.validation_state != "qualified"
            || self.reference_strategy.solver_name.is_none()
            || self.reference_strategy.configuration_sha256.is_none()
        {
            blockers.push("A qualified, versioned internal-flow reference is required.".into());
        }
        blockers
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EngineeringFieldBlob {
    pub n: usize,
    pub velocity: Vec<f32>,
    pub pressure_pa: Vec<f32>,
    pub mask: Vec<f32>,
    pub cp: Vec<f32>,
    pub traction_pa: Vec<f32>,
}

pub fn encode_engineering_field(blob: &EngineeringFieldBlob) -> Result<Vec<u8>, String> {
    let cube = blob
        .n
        .checked_mul(blob.n)
        .and_then(|value| value.checked_mul(blob.n))
        .ok_or_else(|| "Engineering field dimensions overflow.".to_string())?;
    if blob.n < 3
        || blob.velocity.len() != 3 * cube
        || blob.pressure_pa.len() != cube
        || blob.mask.len() != cube
        || blob.cp.len() != cube
        || blob.traction_pa.len() != 3 * cube
    {
        return Err("Engineering field arrays do not match the declared cubic grid.".into());
    }
    if blob
        .velocity
        .iter()
        .chain(&blob.pressure_pa)
        .chain(&blob.mask)
        .chain(&blob.cp)
        .chain(&blob.traction_pa)
        .any(|value| !value.is_finite())
    {
        return Err("Engineering field contains a non-finite value.".into());
    }
    let values = 9usize
        .checked_mul(cube)
        .ok_or_else(|| "Engineering field payload size overflows.".to_string())?;
    let mut bytes = Vec::with_capacity(16 + values * 4);
    bytes.extend_from_slice(b"REYNENG1");
    bytes.extend_from_slice(&(blob.n as u32).to_le_bytes());
    bytes.extend_from_slice(&(values as u32).to_le_bytes());
    for value in blob
        .velocity
        .iter()
        .chain(&blob.pressure_pa)
        .chain(&blob.mask)
        .chain(&blob.cp)
        .chain(&blob.traction_pa)
    {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    Ok(bytes)
}

pub fn decode_engineering_field(bytes: &[u8]) -> Result<EngineeringFieldBlob, String> {
    if bytes.len() < 16 || &bytes[..8] != b"REYNENG1" {
        return Err("Engineering field header is invalid.".into());
    }
    let n = u32::from_le_bytes(bytes[8..12].try_into().expect("four-byte grid")) as usize;
    let values = u32::from_le_bytes(bytes[12..16].try_into().expect("four-byte count")) as usize;
    let cube = n
        .checked_mul(n)
        .and_then(|value| value.checked_mul(n))
        .ok_or_else(|| "Engineering field dimensions overflow.".to_string())?;
    if n < 3 || values != 9 * cube || bytes.len() != 16 + values * 4 {
        return Err("Engineering field byte count does not match its grid.".into());
    }
    let decoded: Vec<f32> = bytes[16..]
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("four-byte f32")))
        .collect();
    if decoded.iter().any(|value| !value.is_finite()) {
        return Err("Engineering field contains a non-finite value.".into());
    }
    Ok(EngineeringFieldBlob {
        n,
        velocity: decoded[..3 * cube].to_vec(),
        pressure_pa: decoded[3 * cube..4 * cube].to_vec(),
        mask: decoded[4 * cube..5 * cube].to_vec(),
        cp: decoded[5 * cube..6 * cube].to_vec(),
        traction_pa: decoded[6 * cube..9 * cube].to_vec(),
    })
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct FeaLoadProvenance {
    pub source_revision_id: String,
    pub case_revision_id: String,
    pub run_id: String,
    pub model_sha256: String,
    pub contract_kind: String,
    pub coordinate_frame: String,
}

const FEA_CSV_HEADER: &str = "x_m,y_m,z_m,traction_x_pa,traction_y_pa,traction_z_pa,cp,source_class,method,schema,source_revision_id,case_revision_id,run_id,model_sha256,contract_kind,coordinate_frame";
const FEA_SOURCE_CLASS: &str = "derived_from_model_prediction_and_recovered_pressure";

fn validate_fea_loads(
    positions: &[[f64; 3]],
    tractions: &[[f64; 3]],
    cp: &[f64],
    provenance: &FeaLoadProvenance,
) -> Result<(), String> {
    if positions.len() != tractions.len() || positions.len() != cp.len() {
        return Err(format!(
            "FEA load arrays must have matching lengths (positions {}, tractions {}, Cp {}).",
            positions.len(),
            tractions.len(),
            cp.len()
        ));
    }
    if positions.is_empty() {
        return Err("FEA load export requires at least one mapped surface-load row.".into());
    }
    for (label, value) in [
        ("source revision", provenance.source_revision_id.as_str()),
        ("case revision", provenance.case_revision_id.as_str()),
        ("run", provenance.run_id.as_str()),
        ("model SHA-256", provenance.model_sha256.as_str()),
        ("contract", provenance.contract_kind.as_str()),
        ("coordinate frame", provenance.coordinate_frame.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("FEA load export requires a persisted {label}."));
        }
        if value.chars().any(char::is_control) {
            return Err(format!(
                "FEA load export {label} must be single-line text without control characters."
            ));
        }
    }
    if provenance.model_sha256.len() != 64
        || !provenance
            .model_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("FEA load export requires a canonical lowercase model SHA-256.".into());
    }
    if provenance.contract_kind != EXTERNAL_FLOW_CONTRACT {
        return Err(format!(
            "FEA load export supports only contract {EXTERNAL_FLOW_CONTRACT}; received {}.",
            provenance.contract_kind
        ));
    }
    let axes = ["x", "y", "z"];
    for (row, ((position, traction), coefficient)) in
        positions.iter().zip(tractions).zip(cp).enumerate()
    {
        for axis in 0..3 {
            if !position[axis].is_finite() {
                return Err(format!(
                    "FEA load row {} has a non-finite {} coordinate.",
                    row + 1,
                    axes[axis]
                ));
            }
            if !traction[axis].is_finite() {
                return Err(format!(
                    "FEA load row {} has a non-finite {} traction.",
                    row + 1,
                    axes[axis]
                ));
            }
        }
        if !coefficient.is_finite() {
            return Err(format!(
                "FEA load row {} has a non-finite Cp value.",
                row + 1
            ));
        }
    }
    Ok(())
}

fn write_csv_field<W: Write>(writer: &mut W, value: &str) -> std::io::Result<()> {
    if !value.bytes().any(|byte| matches!(byte, b',' | b'"')) {
        return writer.write_all(value.as_bytes());
    }
    writer.write_all(b"\"")?;
    let mut start = 0;
    for (index, character) in value.char_indices() {
        if character == '"' {
            writer.write_all(&value.as_bytes()[start..index])?;
            writer.write_all(b"\"\"")?;
            start = index + character.len_utf8();
        }
    }
    writer.write_all(&value.as_bytes()[start..])?;
    writer.write_all(b"\"")
}

/// Stream a versioned, SI-only FEA surface-load CSV to the supplied writer.
///
/// Validation completes before the first byte is written. Numeric formatting is
/// locale-independent, metadata follows RFC 4180 quoting, and no row-sized
/// collection is built internally.
pub fn write_fea_load_csv<W: Write>(
    writer: &mut W,
    positions: &[[f64; 3]],
    tractions: &[[f64; 3]],
    cp: &[f64],
    provenance: &FeaLoadProvenance,
) -> Result<(), String> {
    validate_fea_loads(positions, tractions, cp, provenance)?;
    let write = |result: std::io::Result<()>| {
        result.map_err(|error| format!("FEA load CSV write failed: {error}"))
    };
    write(writeln!(writer, "{FEA_CSV_HEADER}"))?;
    for ((position, traction), coefficient) in positions.iter().zip(tractions).zip(cp) {
        write(write!(
            writer,
            "{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}",
            position[0],
            position[1],
            position[2],
            traction[0],
            traction[1],
            traction[2],
            coefficient,
        ))?;
        for value in [
            FEA_SOURCE_CLASS,
            SURFACE_LOAD_METHOD,
            FEA_LOAD_SCHEMA,
            provenance.source_revision_id.as_str(),
            provenance.case_revision_id.as_str(),
            provenance.run_id.as_str(),
            provenance.model_sha256.as_str(),
            provenance.contract_kind.as_str(),
            provenance.coordinate_frame.as_str(),
        ] {
            write(writer.write_all(b","))?;
            write(write_csv_field(writer, value))?;
        }
        write(writer.write_all(b"\n"))?;
    }
    Ok(())
}

pub fn fea_load_csv(
    positions: &[[f64; 3]],
    tractions: &[[f64; 3]],
    cp: &[f64],
    provenance: &FeaLoadProvenance,
) -> Result<String, String> {
    let mut bytes = Vec::new();
    write_fea_load_csv(&mut bytes, positions, tractions, cp, provenance)?;
    String::from_utf8(bytes).map_err(|error| format!("FEA load CSV encoding failed: {error}"))
}

/// Physical seconds represented by one model horizon step.
///
/// The solver is nondimensionalized with free stream `u = 1` and length
/// `solver_characteristic_length` standing in for the declared reference length,
/// so one solver time unit is `L_ref / (c · U)` seconds and one horizon step is
/// `dt_frame` of them. Returns `None` unless every input is finite and positive,
/// because a horizon step with no stated frame interval has no honest time.
pub fn seconds_per_horizon_step(
    dt_frame: f64,
    solver_characteristic_length: f64,
    reference_length_m: f64,
    velocity_mps: f64,
) -> Option<f64> {
    let inputs = [
        dt_frame,
        solver_characteristic_length,
        reference_length_m,
        velocity_mps,
    ];
    if inputs
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return None;
    }
    Some(dt_frame * reference_length_m / (solver_characteristic_length * velocity_mps))
}

/// Convert a point from the current column-major solver transform back into the
/// imported STL source frame and then apply the approved source-unit scale.
pub fn solver_point_to_source_m(
    solver_point: [f64; 3],
    transform_4x4: [f64; 16],
    meters_per_source_unit: f64,
) -> Result<[f64; 3], String> {
    if !meters_per_source_unit.is_finite() || meters_per_source_unit <= 0.0 {
        return Err("Source-unit conversion must be finite and positive.".into());
    }
    if transform_4x4.iter().any(|component| !component.is_finite()) {
        return Err("Preprocessing transform contains a non-finite value.".into());
    }
    let a = [
        [transform_4x4[0], transform_4x4[4], transform_4x4[8]],
        [transform_4x4[1], transform_4x4[5], transform_4x4[9]],
        [transform_4x4[2], transform_4x4[6], transform_4x4[10]],
    ];
    let determinant = a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
        - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
        + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0]);
    if !determinant.is_finite() || determinant.abs() <= 1e-18 {
        return Err("Preprocessing transform is singular.".into());
    }
    let inverse = [
        [
            (a[1][1] * a[2][2] - a[1][2] * a[2][1]) / determinant,
            (a[0][2] * a[2][1] - a[0][1] * a[2][2]) / determinant,
            (a[0][1] * a[1][2] - a[0][2] * a[1][1]) / determinant,
        ],
        [
            (a[1][2] * a[2][0] - a[1][0] * a[2][2]) / determinant,
            (a[0][0] * a[2][2] - a[0][2] * a[2][0]) / determinant,
            (a[0][2] * a[1][0] - a[0][0] * a[1][2]) / determinant,
        ],
        [
            (a[1][0] * a[2][1] - a[1][1] * a[2][0]) / determinant,
            (a[0][1] * a[2][0] - a[0][0] * a[2][1]) / determinant,
            (a[0][0] * a[1][1] - a[0][1] * a[1][0]) / determinant,
        ],
    ];
    let translated = [
        solver_point[0] - transform_4x4[12],
        solver_point[1] - transform_4x4[13],
        solver_point[2] - transform_4x4[14],
    ];
    Ok(inverse.map(|row| {
        (row[0] * translated[0] + row[1] * translated[1] + row[2] * translated[2])
            * meters_per_source_unit
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operating_point_reports_support_and_units() {
        let mut operating = OperatingPoint::default();
        assert!(operating.reynolds().is_none());
        assert!(!operating.validation(64).is_empty());
        operating.length_unit = LengthUnit::Meter;
        operating.reference_length = 0.002_681_48;
        operating.velocity = 1.0;
        operating.density = 1.225;
        operating.viscosity = 1.81e-5;
        operating.horizon_steps = 64;
        let reynolds = operating.reynolds().unwrap();
        assert!((reynolds - 181.5).abs() < 0.5);
        assert!(operating.validation(64).is_empty());
    }

    #[test]
    fn model_independent_setup_does_not_invent_horizon_support() {
        let mut operating = OperatingPoint::default();
        operating.length_unit = LengthUnit::Millimeter;
        assert!(!operating
            .validation(0)
            .iter()
            .any(|issue| issue.contains("Horizon must lie inside")));

        let unavailable = ModelSupport {
            status: "unavailable".into(),
            ..Default::default()
        };
        assert!(unavailable
            .validation(64)
            .iter()
            .any(|issue| issue.contains("inference is blocked")));
    }

    #[test]
    fn preflight_requires_transform_and_resolved_closed_geometry() {
        let mut preflight = GeometryPreflight {
            source_sha256: "a".repeat(64),
            source_bytes: 1024,
            triangles: 12,
            components: 1,
            source_extents: [1.0; 3],
            proposed_scale: 1.0,
            solver_characteristic_length: 0.6,
            transform_4x4: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            target_grid: 64,
            solid_voxels: 128,
            voxel_components: 1,
            minimum_cells_across: 4,
            boundary_clearance_cells: 6,
            voxel_classification_version: 2,
            ..Default::default()
        };
        assert!(!preflight.ready());
        preflight.transform_approved = true;
        assert!(preflight.ready());
        preflight.boundary_edges = 1;
        assert!(!preflight.ready());
    }

    #[test]
    fn named_waivers_do_not_bypass_hard_gates() {
        let mut preflight = GeometryPreflight {
            source_sha256: "b".repeat(64),
            source_bytes: 1024,
            triangles: 12,
            components: 1,
            source_extents: [1.0; 3],
            proposed_scale: 0.6,
            solver_characteristic_length: 0.6,
            transform_4x4: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            target_grid: 32,
            solid_voxels: 100,
            voxel_components: 1,
            minimum_cells_across: 2,
            boundary_clearance_cells: 4,
            voxel_classification_version: 2,
            transform_approved: true,
            ..Default::default()
        };
        assert!(preflight
            .record_waiver("voxel.under_resolved", "accepted for screening only")
            .is_ok());
        assert!(preflight.ready());
        preflight.solid_voxels = 0;
        assert!(preflight
            .record_waiver("voxel.empty", "accept empty geometry")
            .is_err());
        assert!(!preflight.ready());
    }

    #[test]
    fn measured_geometry_fidelity_can_never_be_waived_by_prose() {
        let mut preflight = GeometryPreflight {
            source_sha256: "f".repeat(64),
            source_bytes: 1024,
            triangles: 12,
            components: 1,
            source_extents: [1.0; 3],
            proposed_scale: 0.6,
            solver_characteristic_length: 0.6,
            transform_4x4: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
            target_grid: 64,
            solid_voxels: 100,
            voxel_components: 1,
            minimum_cells_across: 4,
            boundary_clearance_cells: 4,
            voxel_classification_version: 2,
            voxel_axis_disagreement_fraction: 0.10,
            transform_approved: true,
            ..Default::default()
        };
        assert!(preflight
            .record_waiver("voxel.axis_disagreement", "accepted for screening only")
            .is_err());
        assert!(!preflight.ready());
        preflight.voxel_axis_disagreement_fraction = 0.0;
        preflight.boundary_edges = 1;
        assert!(preflight
            .record_waiver("mesh.open_boundary", "accepted for screening only")
            .is_err());
        assert!(!preflight.ready());
    }

    #[test]
    fn external_contract_records_exact_support_and_rejects_unimplemented_direction() {
        let mut case = ExternalFlowCase {
            stage: CaseStage::Ready,
            case_id: "case-1".into(),
            name: "cube".into(),
            source_name: "cube.stl".into(),
            source_revision_id: Some("source-1".into()),
            case_revision_id: Some("case-revision-1".into()),
            model_id: "flow3d.pth".into(),
            model_sha256: Some("c".repeat(64)),
            model_max_steps: 64,
            model_support: ModelSupport {
                status: "clean".into(),
                dimension: 3,
                grid: 32,
                input_channels: 4,
                output_channels: 3,
                scenario: "obstacle".into(),
                physics_contract: "fixed_body_brinkman.v1".into(),
            },
            preflight: GeometryPreflight {
                source_sha256: "d".repeat(64),
                source_bytes: 2048,
                triangles: 12,
                components: 1,
                source_extents: [1.0; 3],
                proposed_scale: 0.6,
                solver_characteristic_length: 0.6,
                transform_4x4: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
                target_grid: 32,
                solid_voxels: 100,
                voxel_components: 1,
                minimum_cells_across: 4,
                boundary_clearance_cells: 4,
                voxel_classification_version: 2,
                transform_approved: true,
                ..Default::default()
            },
            operating: OperatingPoint {
                length_unit: LengthUnit::Meter,
                reference_length: 0.002_681_48,
                horizon_steps: 4,
                ..Default::default()
            },
            result: None,
            parent_run_id: None,
        };
        assert!(case.ready());
        let contract = case.exact_contract();
        assert_eq!(contract["case_revision_id"], "case-revision-1");
        assert_eq!(contract["model"]["support"]["grid"], 32);
        assert_eq!(contract["surface_load_method"], SURFACE_LOAD_METHOD);
        case.operating.flow_direction = [0.0, 1.0, 0.0];
        assert!(case
            .readiness_issues()
            .iter()
            .any(|issue| issue.contains("+X")));
    }

    #[test]
    fn internal_flow_is_explicitly_blocked() {
        let contract = InternalFlowContract::default();
        assert!(!contract.execution_available);
        assert!(contract.limitation.contains("incompatible"));
        assert!(contract
            .quantities_of_interest
            .contains(&"pressure drop".to_string()));
        let exact = contract.exact_contract();
        assert_eq!(exact["contract_kind"], INTERNAL_FLOW_CONTRACT);
        assert_eq!(exact["reference_strategy"]["required"], true);
        assert_eq!(
            exact["targets"]["mass_flow_balance_tolerance_fraction"],
            0.01
        );
        assert_eq!(
            contract.execution_blockers(),
            vec![contract.limitation.clone()]
        );

        let boundary = |role: &str| InternalBoundaryAssignment {
            region_id: Some(format!("{role}-1")),
            name: role.into(),
            role: role.into(),
            ..InternalBoundaryAssignment::default()
        };
        let mut qualified = contract;
        qualified.execution_available = true;
        qualified.compatible_model_id = Some("qualified-internal-model".into());
        qualified.inlet_assignments = vec![boundary("velocity_inlet")];
        qualified.outlet_assignments = vec![boundary("pressure_outlet")];
        qualified.wall_assignments = vec![boundary("wall")];
        qualified.fluid_properties.density_kg_m3 = Some(1.225);
        qualified.fluid_properties.dynamic_viscosity_pa_s = Some(1.81e-5);
        qualified.reference_strategy.solver_name = Some("reference-solver".into());
        qualified.reference_strategy.configuration_sha256 = Some("a".repeat(64));
        qualified.reference_strategy.validation_state = "qualified".into();
        assert!(qualified.execution_blockers().is_empty());
    }

    fn fea_provenance() -> FeaLoadProvenance {
        FeaLoadProvenance {
            source_revision_id: "source-1".into(),
            case_revision_id: "case-revision-1".into(),
            run_id: "run-1".into(),
            model_sha256: "a".repeat(64),
            contract_kind: EXTERNAL_FLOW_CONTRACT.into(),
            coordinate_frame: "approved_stl_source_frame".into(),
        }
    }

    #[test]
    fn fea_export_is_versioned_and_shape_checked() {
        let provenance = fea_provenance();
        let csv =
            fea_load_csv(&[[1.0, 2.0, 3.0]], &[[4.0, 5.0, 6.0]], &[0.7], &provenance).unwrap();
        assert!(csv.contains(SURFACE_LOAD_METHOD));
        assert!(csv.contains("traction_x_pa"));
        assert!(csv.contains("source-1,case-revision-1,run-1"));
        assert!(csv.contains("1.00000000000000000e0"));
        let error = fea_load_csv(&[], &[[0.0; 3]], &[], &provenance).unwrap_err();
        assert!(error.contains("positions 0, tractions 1, Cp 0"));
        assert!(fea_load_csv(&[], &[], &[], &provenance)
            .unwrap_err()
            .contains("at least one"));
    }

    #[test]
    fn fea_export_quotes_metadata_and_rejects_nonfinite_values() {
        let mut provenance = fea_provenance();
        provenance.source_revision_id = "source,1".into();
        provenance.case_revision_id = "case \"quoted\"".into();
        provenance.coordinate_frame = "approved source, SI".into();
        let csv = fea_load_csv(
            &[[1.25, 2.5, 3.75]],
            &[[4.0, 5.0, 6.0]],
            &[0.7],
            &provenance,
        )
        .unwrap();
        assert!(csv.contains("\"source,1\""));
        assert!(csv.contains("\"case \"\"quoted\"\"\""));
        assert!(csv.contains("\"approved source, SI\""));
        assert!(csv.contains("1.25000000000000000e0"));
        assert!(!csv.contains("1,25000000000000000e0"));

        let mut output = b"unchanged".to_vec();
        let error = write_fea_load_csv(
            &mut output,
            &[[f64::NAN, 0.0, 0.0]],
            &[[0.0; 3]],
            &[0.0],
            &provenance,
        )
        .unwrap_err();
        assert!(error.contains("row 1"));
        assert!(error.contains("non-finite x coordinate"));
        assert_eq!(output, b"unchanged");

        let error = fea_load_csv(
            &[[0.0; 3]],
            &[[0.0, f64::INFINITY, 0.0]],
            &[0.0],
            &provenance,
        )
        .unwrap_err();
        assert!(error.contains("non-finite y traction"));
        assert!(
            fea_load_csv(&[[0.0; 3]], &[[0.0; 3]], &[f64::NAN], &provenance)
                .unwrap_err()
                .contains("non-finite Cp")
        );
    }

    #[test]
    fn fea_export_rejects_invalid_provenance_and_streams_to_writer() {
        let mut provenance = fea_provenance();
        provenance.contract_kind = INTERNAL_FLOW_CONTRACT.into();
        assert!(fea_load_csv(&[[0.0; 3]], &[[0.0; 3]], &[0.0], &provenance)
            .unwrap_err()
            .contains(EXTERNAL_FLOW_CONTRACT));
        provenance = fea_provenance();
        provenance.model_sha256 = "A".repeat(64);
        assert!(fea_load_csv(&[[0.0; 3]], &[[0.0; 3]], &[0.0], &provenance)
            .unwrap_err()
            .contains("canonical lowercase"));
        provenance = fea_provenance();

        #[derive(Default)]
        struct ByteCounter(usize);
        impl Write for ByteCounter {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0 += bytes.len();
                Ok(bytes.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let positions = vec![[1.0, 2.0, 3.0]; 4_096];
        let tractions = vec![[4.0, 5.0, 6.0]; positions.len()];
        let cp = vec![0.7; positions.len()];
        let mut counter = ByteCounter::default();
        write_fea_load_csv(&mut counter, &positions, &tractions, &cp, &provenance).unwrap();
        assert!(counter.0 > positions.len() * 200);

        struct FailingWriter;
        impl Write for FailingWriter {
            fn write(&mut self, _bytes: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("simulated full disk"))
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        assert!(write_fea_load_csv(
            &mut FailingWriter,
            &positions[..1],
            &tractions[..1],
            &cp[..1],
            &provenance,
        )
        .unwrap_err()
        .contains("simulated full disk"));
    }

    #[test]
    fn solver_points_round_trip_to_approved_source_frame() {
        let transform = [
            2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 10.0, 20.0, 30.0, 1.0,
        ];
        let point = solver_point_to_source_m([12.0, 24.0, 36.0], transform, 1e-3).unwrap();
        assert_eq!(point, [0.001, 0.002, 0.003]);
    }

    #[test]
    fn engineering_field_binary_round_trip_is_strict() {
        let n = 3;
        let cube = n * n * n;
        let blob = EngineeringFieldBlob {
            n,
            velocity: vec![1.0; 3 * cube],
            pressure_pa: vec![2.0; cube],
            mask: vec![0.5; cube],
            cp: vec![3.0; cube],
            traction_pa: vec![4.0; 3 * cube],
        };
        let bytes = encode_engineering_field(&blob).unwrap();
        assert_eq!(decode_engineering_field(&bytes).unwrap(), blob);
        assert!(decode_engineering_field(&bytes[..bytes.len() - 1]).is_err());
        let mut invalid = blob;
        invalid.cp[0] = f32::NAN;
        assert!(encode_engineering_field(&invalid).is_err());
    }

    fn draft_case() -> ExternalFlowCase {
        ExternalFlowCase {
            stage: CaseStage::Results,
            case_id: "case-draft".into(),
            name: "draft".into(),
            source_name: "draft.stl".into(),
            source_revision_id: Some("source-draft".into()),
            case_revision_id: Some("revision-draft".into()),
            model_id: "model-draft".into(),
            model_sha256: Some("a".repeat(64)),
            model_max_steps: 16,
            model_support: ModelSupport::default(),
            preflight: GeometryPreflight {
                source_sha256: "b".repeat(64),
                transform_approved: true,
                waivers: vec!["voxel.under_resolved: screening only".into()],
                ..Default::default()
            },
            operating: OperatingPoint {
                length_unit: LengthUnit::Meter,
                velocity: 10.0,
                ..Default::default()
            },
            result: Some(EngineeringResult {
                method: SURFACE_LOAD_METHOD.into(),
                ..Default::default()
            }),
            parent_run_id: Some("run-parent".into()),
        }
    }

    fn draft_scope() -> CaseDraftScope {
        CaseDraftScope::new(
            "project-draft",
            "case-draft",
            Some("source-draft".into()),
            "b".repeat(64),
        )
    }

    fn snapshot_with_velocity(velocity: f64) -> CaseDraftSnapshot {
        let mut case = draft_case();
        case.operating.velocity = velocity;
        CaseDraftSnapshot::capture(&case)
    }

    #[test]
    fn case_draft_history_obeys_stack_semantics() {
        let scope = draft_scope();
        let a = snapshot_with_velocity(10.0);
        let b = snapshot_with_velocity(20.0);
        let c = snapshot_with_velocity(30.0);
        let mut history = CaseDraftHistory::default();

        assert!(history.record_change(scope.clone(), a.clone(), &b, false));
        assert!(history.record_change(scope.clone(), b.clone(), &c, false));
        assert_eq!(history.undo(scope.clone(), c.clone()), Some(b.clone()));
        assert_eq!(history.undo(scope.clone(), b.clone()), Some(a.clone()));
        assert_eq!(history.redo(scope.clone(), a.clone()), Some(b.clone()));
        assert_eq!(history.redo(scope, b), Some(c));
    }

    #[test]
    fn new_case_draft_edit_invalidates_redo() {
        let scope = draft_scope();
        let a = snapshot_with_velocity(10.0);
        let b = snapshot_with_velocity(20.0);
        let fork = snapshot_with_velocity(15.0);
        let mut history = CaseDraftHistory::default();

        history.record_change(scope.clone(), a.clone(), &b, false);
        assert_eq!(history.undo(scope.clone(), b), Some(a.clone()));
        history.record_change(scope.clone(), a, &fork, false);
        assert_eq!(history.redo(scope, fork), None);
    }

    #[test]
    fn case_draft_history_is_bounded_and_coalesces_active_edits() {
        let scope = draft_scope();
        let mut history = CaseDraftHistory::with_limit(3);
        let mut current = snapshot_with_velocity(0.0);
        for velocity in 1..=8 {
            let next = snapshot_with_velocity(velocity as f64);
            history.record_change(scope.clone(), current, &next, false);
            current = next;
        }
        assert_eq!(history.undo.len(), 3);
        assert_eq!(
            history.undo(scope.clone(), current),
            Some(snapshot_with_velocity(7.0))
        );

        history.rebase(scope.clone());
        let a = snapshot_with_velocity(10.0);
        let b = snapshot_with_velocity(11.0);
        let c = snapshot_with_velocity(12.0);
        history.record_change(scope.clone(), a.clone(), &b, false);
        history.record_change(scope.clone(), b, &c, true);
        assert_eq!(history.undo(scope, c), Some(a));
    }

    #[test]
    fn case_or_source_scope_transition_rebases_history() {
        let scope = draft_scope();
        let a = snapshot_with_velocity(10.0);
        let b = snapshot_with_velocity(20.0);
        let mut history = CaseDraftHistory::default();
        history.record_change(scope.clone(), a, &b, false);
        assert!(history.can_undo(&scope));

        let next_scope = CaseDraftScope::new(
            "project-draft",
            "case-draft",
            Some("source-next".into()),
            "c".repeat(64),
        );
        history.rebase(next_scope.clone());

        assert!(!history.can_undo(&next_scope));
        assert!(!history.can_redo(&next_scope));
        assert_eq!(history.undo(next_scope, b), None);
    }

    #[test]
    fn restoring_draft_never_mutates_evidence_or_identity() {
        let original = draft_case();
        let snapshot = CaseDraftSnapshot::capture(&original);
        let mut edited = original.clone();
        edited.operating.velocity = 42.0;
        edited.preflight.transform_approved = false;
        edited.preflight.waivers.clear();
        edited.case_id = "case-current".into();
        edited.case_revision_id = Some("revision-current".into());
        edited.source_revision_id = Some("source-current".into());
        edited.preflight.source_sha256 = "c".repeat(64);
        edited.preflight.source_bytes = 4096;
        edited.preflight.transform_4x4 = [2.0; 16];
        edited.model_id = "model-current".into();
        edited.model_sha256 = Some("d".repeat(64));
        edited.model_max_steps = 99;
        edited.model_support.status = "identity-current".into();
        edited.result = Some(EngineeringResult {
            method: "immutable-result".into(),
            ..Default::default()
        });
        edited.parent_run_id = Some("run-current".into());
        let immutable_current = (
            edited.case_id.clone(),
            edited.case_revision_id.clone(),
            edited.source_revision_id.clone(),
            edited.preflight.source_sha256.clone(),
            edited.preflight.source_bytes,
            edited.preflight.transform_4x4,
            edited.model_id.clone(),
            edited.model_sha256.clone(),
            edited.model_max_steps,
            edited.model_support.clone(),
            edited.result.clone(),
            edited.parent_run_id.clone(),
        );

        snapshot.restore(&mut edited);

        assert_eq!(edited.operating, original.operating);
        assert_eq!(
            (
                edited.case_id,
                edited.case_revision_id,
                edited.source_revision_id,
                edited.preflight.source_sha256,
                edited.preflight.source_bytes,
                edited.preflight.transform_4x4,
                edited.model_id,
                edited.model_sha256,
                edited.model_max_steps,
                edited.model_support,
                edited.result,
                edited.parent_run_id,
            ),
            immutable_current
        );
    }
}
