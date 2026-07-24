//! Typed N5 Benchmark Inspector maps.
//!
//! The Python sidecar ships one immutable evidence tensor for a selected cell:
//! `[variable, model/reference/error, y, x]`.  This module validates that protocol
//! and exposes calibrated scales, so changing the UI mode is instant and cannot
//! accidentally compare panels with independent color normalization.

use serde::{Deserialize, Serialize};

pub const INSPECTOR_SCHEMA: &str = "reyn.benchmark-inspector.maps.v2";
pub const INSPECTOR_PROTOCOL_VERSION: u64 = 2;
pub const INSPECTOR_LAYOUT: &str = "variable,model_reference_error,y,x";
pub const INSPECTOR_DOMAIN: &str = "periodic_2pi";
pub const INSPECTOR_DERIVATIVE: &str = "fourier_spectral_nyquist_zero";
pub const INSPECTOR_PRESSURE: &str = "advective_poisson_density_normalized_zero_mean";

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectorVariable {
    #[default]
    Velocity,
    Vorticity,
    Pressure,
    Divergence,
}

impl InspectorVariable {
    pub const ALL: [Self; 4] = [
        Self::Velocity,
        Self::Vorticity,
        Self::Pressure,
        Self::Divergence,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Velocity => "velocity",
            Self::Vorticity => "vorticity",
            Self::Pressure => "pressure",
            Self::Divergence => "divergence",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Velocity => "Velocity",
            Self::Vorticity => "Vorticity",
            Self::Pressure => "Recovered pressure",
            Self::Divergence => "Spatial divergence",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Velocity => "|u|",
            Self::Vorticity => "ω",
            Self::Pressure => "p/ρ",
            Self::Divergence => "∇·u",
        }
    }

    pub fn error_symbol(self) -> &'static str {
        match self {
            Self::Velocity => "|Δu|",
            Self::Vorticity => "Δω",
            Self::Pressure => "Δ(p/ρ)",
            Self::Divergence => "Δ(∇·u)",
        }
    }

    pub fn unit_key(self) -> &'static str {
        match self {
            Self::Velocity => "solver_velocity_unit",
            Self::Vorticity | Self::Divergence => "inverse_solver_time_unit",
            Self::Pressure => "solver_velocity_unit_squared",
        }
    }

    pub fn unit_label(self) -> &'static str {
        match self {
            Self::Velocity => "solver velocity unit",
            Self::Vorticity | Self::Divergence => "solver time⁻¹",
            Self::Pressure => "solver velocity² · density-normalized",
        }
    }

    pub fn model_source(self) -> &'static str {
        match self {
            Self::Velocity => "MODEL",
            Self::Vorticity | Self::Divergence => "DERIVED_FROM_MODEL",
            Self::Pressure => "RECOVERED_FROM_MODEL",
        }
    }

    pub fn reference_source(self) -> &'static str {
        match self {
            Self::Velocity => "SOLVER_REFERENCE",
            Self::Vorticity | Self::Divergence => "DERIVED_FROM_SOLVER_REFERENCE",
            Self::Pressure => "RECOVERED_FROM_SOLVER_REFERENCE",
        }
    }

    pub fn model_source_label(self) -> &'static str {
        match self {
            Self::Velocity => "MODEL",
            Self::Vorticity | Self::Divergence => "DERIVED · MODEL",
            Self::Pressure => "RECOVERED · MODEL",
        }
    }

    pub fn reference_source_label(self) -> &'static str {
        match self {
            Self::Velocity => "SOLVER REFERENCE",
            Self::Vorticity | Self::Divergence => "DERIVED · REFERENCE",
            Self::Pressure => "RECOVERED · REFERENCE",
        }
    }

    pub fn signed(self) -> bool {
        !matches!(self, Self::Velocity)
    }

    pub fn method_note(self) -> &'static str {
        match self {
            Self::Velocity => "pointwise |u| · error is |u_model − u_reference|",
            Self::Vorticity => "derived Fourier-spectral curl · periodic 2π domain",
            Self::Pressure => "recovered by advective Poisson · density-normalized · zero-mean",
            Self::Divergence => "derived Fourier-spectral pointwise ∂u/∂x + ∂v/∂y",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|variable| variable.key() == key)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VariableMaps {
    pub variable: InspectorVariable,
    pub model: Vec<f32>,
    pub reference: Vec<f32>,
    pub error: Vec<f32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct InspectorMaps {
    n: usize,
    variables: Vec<VariableMaps>,
}

impl InspectorMaps {
    pub fn from_protocol(
        schema: &str,
        n: usize,
        variable_keys: &[String],
        signed: &[bool],
        values: &[f32],
    ) -> Result<Self, String> {
        if schema != INSPECTOR_SCHEMA {
            return Err(format!("unsupported inspector evidence schema: {schema}"));
        }
        Self::from_flat(n, variable_keys, signed, values)
    }

    pub fn from_flat(
        n: usize,
        variable_keys: &[String],
        signed: &[bool],
        values: &[f32],
    ) -> Result<Self, String> {
        if n == 0 {
            return Err("inspector map grid is empty".into());
        }
        if variable_keys.len() != InspectorVariable::ALL.len()
            || signed.len() != variable_keys.len()
        {
            return Err("inspector variable metadata is incomplete".into());
        }
        let expected_keys: Vec<_> = InspectorVariable::ALL
            .iter()
            .map(|variable| variable.key())
            .collect();
        if variable_keys.iter().map(String::as_str).collect::<Vec<_>>() != expected_keys {
            return Err("inspector variables are missing, duplicated, or out of order".into());
        }

        let plane = n
            .checked_mul(n)
            .ok_or_else(|| "inspector map grid overflows".to_string())?;
        let expected_values = variable_keys
            .len()
            .checked_mul(3)
            .and_then(|panels| panels.checked_mul(plane))
            .ok_or_else(|| "inspector payload size overflows".to_string())?;
        if values.len() != expected_values {
            return Err("inspector payload length does not match its map layout".into());
        }
        if values.iter().any(|value| !value.is_finite()) {
            return Err("inspector payload contains non-finite evidence".into());
        }

        let mut variables = Vec::with_capacity(variable_keys.len());
        for (index, (key, &reported_signed)) in variable_keys.iter().zip(signed).enumerate() {
            let variable = InspectorVariable::from_key(key)
                .ok_or_else(|| format!("unknown inspector variable: {key}"))?;
            if reported_signed != variable.signed() {
                return Err(format!("invalid signedness for inspector variable {key}"));
            }
            let start = index * 3 * plane;
            variables.push(VariableMaps {
                variable,
                model: values[start..start + plane].to_vec(),
                reference: values[start + plane..start + 2 * plane].to_vec(),
                error: values[start + 2 * plane..start + 3 * plane].to_vec(),
            });
        }
        Ok(Self { n, variables })
    }

    pub fn n(&self) -> usize {
        self.n
    }

    pub fn get(&self, variable: InspectorVariable) -> Option<&VariableMaps> {
        self.variables.iter().find(|maps| maps.variable == variable)
    }

    /// Shared model/reference scale and a separate error scale.
    pub fn scales(&self, variable: InspectorVariable) -> Option<(f32, f32)> {
        let maps = self.get(variable)?;
        let magnitude = |value: f32| {
            if variable.signed() {
                value.abs()
            } else {
                value
            }
        };
        let comparison = maps
            .model
            .iter()
            .chain(&maps.reference)
            .copied()
            .map(magnitude)
            .fold(1e-6f32, f32::max);
        let error = maps
            .error
            .iter()
            .copied()
            .map(magnitude)
            .fold(1e-6f32, f32::max);
        Some((comparison, error))
    }

    pub fn error_stats(&self, variable: InspectorVariable) -> Option<(f32, f32, f32)> {
        let maps = self.get(variable)?;
        let mut absolute: Vec<f32> = maps.error.iter().map(|value| value.abs()).collect();
        absolute.sort_by(f32::total_cmp);
        let mean = absolute.iter().sum::<f32>() / absolute.len() as f32;
        let p95_index = ((absolute.len() - 1) as f32 * 0.95).round() as usize;
        Some((mean, absolute[p95_index], *absolute.last()?))
    }

    pub fn rms(&self, variable: InspectorVariable) -> Option<(f32, f32, f32)> {
        let maps = self.get(variable)?;
        let rms = |values: &[f32]| {
            (values.iter().map(|value| value * value).sum::<f32>() / values.len() as f32).sqrt()
        };
        Some((rms(&maps.model), rms(&maps.reference), rms(&maps.error)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata() -> (Vec<String>, Vec<bool>) {
        (
            InspectorVariable::ALL
                .iter()
                .map(|variable| variable.key().to_owned())
                .collect(),
            InspectorVariable::ALL
                .iter()
                .map(|variable| variable.signed())
                .collect(),
        )
    }

    #[test]
    fn parses_variable_major_map_layout() {
        let (keys, signed) = metadata();
        let values: Vec<f32> = (0..48).map(|value| value as f32).collect();
        let maps = InspectorMaps::from_flat(2, &keys, &signed, &values).unwrap();

        assert_eq!(maps.n(), 2);
        let divergence = maps.get(InspectorVariable::Divergence).unwrap();
        assert_eq!(divergence.model, vec![36.0, 37.0, 38.0, 39.0]);
        assert_eq!(divergence.error, vec![44.0, 45.0, 46.0, 47.0]);
    }

    #[test]
    fn signed_scales_are_symmetric_and_shared() {
        let (keys, signed) = metadata();
        let mut values = vec![0.0; 48];
        let pressure_start = 2 * 3 * 4;
        values[pressure_start] = -7.0;
        values[pressure_start + 4] = 5.0;
        values[pressure_start + 8] = -3.0;
        let maps = InspectorMaps::from_flat(2, &keys, &signed, &values).unwrap();

        assert_eq!(maps.scales(InspectorVariable::Pressure), Some((7.0, 3.0)));
        assert_eq!(maps.rms(InspectorVariable::Pressure), Some((3.5, 2.5, 1.5)));
        assert_eq!(InspectorVariable::Pressure.label(), "Recovered pressure");
        assert_eq!(
            InspectorVariable::Pressure.reference_source(),
            "RECOVERED_FROM_SOLVER_REFERENCE"
        );
        assert_eq!(
            InspectorVariable::Pressure.unit_key(),
            "solver_velocity_unit_squared"
        );
    }

    #[test]
    fn rejects_short_nonfinite_or_mislabeled_evidence() {
        let (keys, signed) = metadata();
        let values = vec![0.0; 48];
        assert!(InspectorMaps::from_protocol("unknown", 2, &keys, &signed, &values).is_err());
        assert!(InspectorMaps::from_flat(2, &keys, &signed, &values[..47]).is_err());

        let mut nonfinite = values.clone();
        nonfinite[3] = f32::NAN;
        assert!(InspectorMaps::from_flat(2, &keys, &signed, &nonfinite).is_err());

        let mut wrong_signed = signed;
        wrong_signed[0] = true;
        assert!(InspectorMaps::from_flat(2, &keys, &wrong_signed, &values).is_err());

        let mut wrong_order = keys;
        wrong_order.swap(0, 1);
        assert!(
            InspectorMaps::from_flat(2, &wrong_order, &[true, false, true, true], &values).is_err()
        );
    }
}
