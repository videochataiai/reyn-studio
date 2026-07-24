//! Case-centered engineering workflow state.
//!
//! This module deliberately separates supported external-flow analysis from
//! developer demonstrations. It contains no renderer or engine assumptions, so
//! project persistence and scientific gates remain testable without a GPU.

use serde::{Deserialize, Serialize};

pub const EXTERNAL_FLOW_CONTRACT: &str = "external_fixed_body.v1";
pub const INTERNAL_FLOW_CONTRACT: &str = "internal_flow.reference_only.v1";
pub const SURFACE_LOAD_METHOD: &str = "diffuse_interface_traction.v1";
pub const ENGINEERING_RESULT_SCHEMA: &str = "engineering_result.v1";
pub const ENGINEERING_FIELD_SCHEMA: &str = "engineering_field.f32le.v1";
pub const FEA_LOAD_SCHEMA: &str = "reyn_fea_surface_loads.v1";

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
        if self.horizon_steps == 0 || self.horizon_steps > max_steps {
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
    pub source_extents: [f64; 3],
    pub proposed_scale: f64,
    pub solver_characteristic_length: f64,
    pub transform_4x4: [f64; 16],
    pub target_grid: usize,
    pub solid_voxels: usize,
    pub voxel_components: usize,
    pub minimum_cells_across: usize,
    pub boundary_clearance_cells: usize,
    pub warnings: Vec<String>,
    pub waivers: Vec<String>,
    pub transform_approved: bool,
}

impl GeometryPreflight {
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
                    "{} open boundary edges indicate a non-watertight source.",
                    self.boundary_edges
                ),
                true,
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
        if self.components > 1 {
            issue(
                "mesh.disconnected_components",
                format!(
                    "The STL contains {} disconnected surface components.",
                    self.components
                ),
                true,
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
        if self.status == "invalid" || self.status.trim().is_empty() {
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

pub fn fea_load_csv(
    positions: &[[f64; 3]],
    tractions: &[[f64; 3]],
    cp: &[f64],
    provenance: &FeaLoadProvenance,
) -> Result<String, String> {
    if positions.len() != tractions.len() || positions.len() != cp.len() {
        return Err("FEA load arrays must have matching lengths".into());
    }
    for (label, value) in [
        ("source revision", provenance.source_revision_id.as_str()),
        ("case revision", provenance.case_revision_id.as_str()),
        ("run", provenance.run_id.as_str()),
        ("model SHA-256", provenance.model_sha256.as_str()),
        ("contract", provenance.contract_kind.as_str()),
        ("coordinate frame", provenance.coordinate_frame.as_str()),
    ] {
        if value.trim().is_empty() || value.contains([',', '\n', '\r']) {
            return Err(format!("FEA export requires a CSV-safe {label}"));
        }
    }
    if provenance.model_sha256.len() != 64
        || !provenance
            .model_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("FEA export requires a canonical model SHA-256".into());
    }
    let mut csv = String::from(
        "x_m,y_m,z_m,traction_x_pa,traction_y_pa,traction_z_pa,cp,source_class,method,schema,source_revision_id,case_revision_id,run_id,model_sha256,contract_kind,coordinate_frame\n",
    );
    for ((position, traction), coefficient) in positions.iter().zip(tractions).zip(cp.iter()) {
        csv.push_str(&format!(
            "{:.9e},{:.9e},{:.9e},{:.9e},{:.9e},{:.9e},{:.9e},derived_from_model_prediction_and_recovered_pressure,{},{},{},{},{},{},{},{}\n",
            position[0],
            position[1],
            position[2],
            traction[0],
            traction[1],
            traction[2],
            coefficient,
            SURFACE_LOAD_METHOD,
            FEA_LOAD_SCHEMA,
            provenance.source_revision_id,
            provenance.case_revision_id,
            provenance.run_id,
            provenance.model_sha256,
            provenance.contract_kind,
            provenance.coordinate_frame,
        ));
    }
    Ok(csv)
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

    #[test]
    fn fea_export_is_versioned_and_shape_checked() {
        let provenance = FeaLoadProvenance {
            source_revision_id: "source-1".into(),
            case_revision_id: "case-revision-1".into(),
            run_id: "run-1".into(),
            model_sha256: "a".repeat(64),
            contract_kind: EXTERNAL_FLOW_CONTRACT.into(),
            coordinate_frame: "approved_stl_source_frame".into(),
        };
        let csv =
            fea_load_csv(&[[1.0, 2.0, 3.0]], &[[4.0, 5.0, 6.0]], &[0.7], &provenance).unwrap();
        assert!(csv.contains(SURFACE_LOAD_METHOD));
        assert!(csv.contains("traction_x_pa"));
        assert!(csv.contains("source-1,case-revision-1,run-1"));
        assert!(fea_load_csv(&[], &[[0.0; 3]], &[], &provenance).is_err());
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
}
