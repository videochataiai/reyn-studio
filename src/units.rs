//! Display-side unit systems, per-field input units, and numeric formatting.
//!
//! Storage, evidence, run manifests, and the versioned FEA CSV schema remain
//! strictly SI — these conversions exist only at the display/input boundary,
//! so provenance never depends on a UI preference. Every conversion factor is
//! an exact legal definition (international foot, avoirdupois pound).

use serde::{Deserialize, Serialize};

// Exact definitions.
const METERS_PER_FOOT: f64 = 0.3048;
const KG_PER_LBM: f64 = 0.453_592_37;
const N_PER_LBF: f64 = 4.448_221_615_260_5;
const PA_PER_PSI: f64 = N_PER_LBF / (0.0254 * 0.0254); // ≈ 6894.757
const NM_PER_LBFFT: f64 = N_PER_LBF * METERS_PER_FOOT; // ≈ 1.355818
const KGM3_PER_LBMFT3: f64 = KG_PER_LBM / (METERS_PER_FOOT * METERS_PER_FOOT * METERS_PER_FOOT);
const PAS_PER_LBM_FTS: f64 = KG_PER_LBM / METERS_PER_FOOT; // lbm/(ft·s)
const M2_PER_FT2: f64 = METERS_PER_FOOT * METERS_PER_FOOT;

// ---------------------------------------------------------------------------
// Unit system for displayed results
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UnitSystem {
    #[default]
    Si,
    Imperial,
}

impl UnitSystem {
    pub const ALL: [Self; 2] = [Self::Si, Self::Imperial];

    pub fn label(self) -> &'static str {
        match self {
            Self::Si => "SI (m, m/s, Pa, N)",
            Self::Imperial => "Imperial (ft, ft/s, psi, lbf)",
        }
    }
}

/// Physical quantities that appear in displayed results.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quantity {
    Length,
    Velocity,
    Pressure,
    Force,
    Moment,
    Density,
    Viscosity,
    Area,
}

/// Convert an SI value into the display system: `(value, unit symbol)`.
pub fn display_value(quantity: Quantity, si: f64, system: UnitSystem) -> (f64, &'static str) {
    match system {
        UnitSystem::Si => match quantity {
            Quantity::Length => (si, "m"),
            Quantity::Velocity => (si, "m/s"),
            Quantity::Pressure => (si, "Pa"),
            Quantity::Force => (si, "N"),
            Quantity::Moment => (si, "N·m"),
            Quantity::Density => (si, "kg/m³"),
            Quantity::Viscosity => (si, "Pa·s"),
            Quantity::Area => (si, "m²"),
        },
        UnitSystem::Imperial => match quantity {
            Quantity::Length => (si / METERS_PER_FOOT, "ft"),
            Quantity::Velocity => (si / METERS_PER_FOOT, "ft/s"),
            Quantity::Pressure => (si / PA_PER_PSI, "psi"),
            Quantity::Force => (si / N_PER_LBF, "lbf"),
            Quantity::Moment => (si / NM_PER_LBFFT, "lbf·ft"),
            Quantity::Density => (si / KGM3_PER_LBMFT3, "lbm/ft³"),
            Quantity::Viscosity => (si / PAS_PER_LBM_FTS, "lbm/(ft·s)"),
            Quantity::Area => (si / M2_PER_FT2, "ft²"),
        },
    }
}

// ---------------------------------------------------------------------------
// Numeric formatting
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NumberNotation {
    /// Fixed inside [1e-3, 1e5); scientific outside.
    #[default]
    Auto,
    Fixed,
    Scientific,
}

impl NumberNotation {
    pub const ALL: [Self; 3] = [Self::Auto, Self::Fixed, Self::Scientific];

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Automatic",
            Self::Fixed => "Fixed decimal",
            Self::Scientific => "Scientific",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ValueFormat {
    pub significant_digits: u8,
    pub notation: NumberNotation,
}

impl Default for ValueFormat {
    fn default() -> Self {
        Self {
            significant_digits: 5,
            notation: NumberNotation::Auto,
        }
    }
}

pub const MIN_SIGNIFICANT_DIGITS: u8 = 3;
pub const MAX_SIGNIFICANT_DIGITS: u8 = 8;

/// Format a value with the requested significant digits and notation.
pub fn format_value(value: f64, format: ValueFormat) -> String {
    let digits = format
        .significant_digits
        .clamp(MIN_SIGNIFICANT_DIGITS, MAX_SIGNIFICANT_DIGITS) as usize;
    if !value.is_finite() {
        return format!("{value}");
    }
    let scientific = match format.notation {
        NumberNotation::Scientific => true,
        NumberNotation::Fixed => false,
        NumberNotation::Auto => {
            let magnitude = value.abs();
            magnitude != 0.0 && !(1e-3..1e5).contains(&magnitude)
        }
    };
    if scientific {
        format!("{value:.precision$e}", precision = digits - 1)
    } else {
        format_fixed_significant(value, digits)
    }
}

/// Fixed-notation rendering that keeps `digits` significant digits.
fn format_fixed_significant(value: f64, digits: usize) -> String {
    if value == 0.0 {
        return format!("{value:.*}", digits.saturating_sub(1));
    }
    let magnitude = value.abs().log10().floor() as i64;
    let decimals = (digits as i64 - 1 - magnitude).clamp(0, 12) as usize;
    format!("{value:.decimals$}")
}

/// Convert an SI value into the display system and format it as
/// `"value symbol"` — the one-call helper for measurement rows.
pub fn format_quantity(
    quantity: Quantity,
    si: f64,
    system: UnitSystem,
    format: ValueFormat,
) -> String {
    let (value, symbol) = display_value(quantity, si, system);
    format!("{} {}", format_value(value, format), symbol)
}

// ---------------------------------------------------------------------------
// Per-field input units (Case Setup entry) — storage stays SI.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VelocityUnit {
    #[default]
    MetersPerSecond,
    KilometersPerHour,
    MilesPerHour,
    FeetPerSecond,
    Knots,
}

impl VelocityUnit {
    pub const ALL: [Self; 5] = [
        Self::MetersPerSecond,
        Self::KilometersPerHour,
        Self::MilesPerHour,
        Self::FeetPerSecond,
        Self::Knots,
    ];

    pub fn symbol(self) -> &'static str {
        match self {
            Self::MetersPerSecond => "m/s",
            Self::KilometersPerHour => "km/h",
            Self::MilesPerHour => "mph",
            Self::FeetPerSecond => "ft/s",
            Self::Knots => "kn",
        }
    }

    fn si_per_unit(self) -> f64 {
        match self {
            Self::MetersPerSecond => 1.0,
            Self::KilometersPerHour => 1.0 / 3.6,
            Self::MilesPerHour => 0.447_04,
            Self::FeetPerSecond => METERS_PER_FOOT,
            Self::Knots => 1852.0 / 3600.0,
        }
    }

    pub fn to_si(self, value: f64) -> f64 {
        value * self.si_per_unit()
    }

    pub fn from_si(self, si: f64) -> f64 {
        si / self.si_per_unit()
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PressureUnit {
    #[default]
    Pascal,
    Kilopascal,
    Psi,
    Atmosphere,
}

impl PressureUnit {
    pub const ALL: [Self; 4] = [Self::Pascal, Self::Kilopascal, Self::Psi, Self::Atmosphere];

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Pascal => "Pa",
            Self::Kilopascal => "kPa",
            Self::Psi => "psi",
            Self::Atmosphere => "atm",
        }
    }

    fn si_per_unit(self) -> f64 {
        match self {
            Self::Pascal => 1.0,
            Self::Kilopascal => 1000.0,
            Self::Psi => PA_PER_PSI,
            Self::Atmosphere => 101_325.0,
        }
    }

    pub fn to_si(self, value: f64) -> f64 {
        value * self.si_per_unit()
    }

    pub fn from_si(self, si: f64) -> f64 {
        si / self.si_per_unit()
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DensityUnit {
    #[default]
    KilogramsPerCubicMeter,
    PoundsMassPerCubicFoot,
}

impl DensityUnit {
    pub const ALL: [Self; 2] = [Self::KilogramsPerCubicMeter, Self::PoundsMassPerCubicFoot];

    pub fn symbol(self) -> &'static str {
        match self {
            Self::KilogramsPerCubicMeter => "kg/m³",
            Self::PoundsMassPerCubicFoot => "lbm/ft³",
        }
    }

    fn si_per_unit(self) -> f64 {
        match self {
            Self::KilogramsPerCubicMeter => 1.0,
            Self::PoundsMassPerCubicFoot => KGM3_PER_LBMFT3,
        }
    }

    pub fn to_si(self, value: f64) -> f64 {
        value * self.si_per_unit()
    }

    pub fn from_si(self, si: f64) -> f64 {
        si / self.si_per_unit()
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ViscosityUnit {
    #[default]
    PascalSecond,
    /// mPa·s ≡ centipoise; water ≈ 1 cP.
    MillipascalSecond,
}

impl ViscosityUnit {
    pub const ALL: [Self; 2] = [Self::PascalSecond, Self::MillipascalSecond];

    pub fn symbol(self) -> &'static str {
        match self {
            Self::PascalSecond => "Pa·s",
            Self::MillipascalSecond => "mPa·s",
        }
    }

    fn si_per_unit(self) -> f64 {
        match self {
            Self::PascalSecond => 1.0,
            Self::MillipascalSecond => 1e-3,
        }
    }

    pub fn to_si(self, value: f64) -> f64 {
        value * self.si_per_unit()
    }

    pub fn from_si(self, si: f64) -> f64 {
        si / self.si_per_unit()
    }
}

/// Shared surface of the per-field input units so the Case Setup UI can draw
/// one generic unit-aware control for every field.
pub trait InputUnit: Copy + PartialEq + Sized + 'static {
    fn all() -> &'static [Self];
    fn unit_symbol(self) -> &'static str;
    fn unit_to_si(self, value: f64) -> f64;
    fn unit_from_si(self, si: f64) -> f64;
}

macro_rules! impl_input_unit {
    ($unit:ty) => {
        impl InputUnit for $unit {
            fn all() -> &'static [Self] {
                &Self::ALL
            }
            fn unit_symbol(self) -> &'static str {
                self.symbol()
            }
            fn unit_to_si(self, value: f64) -> f64 {
                self.to_si(value)
            }
            fn unit_from_si(self, si: f64) -> f64 {
                self.from_si(si)
            }
        }
    };
}

impl_input_unit!(VelocityUnit);
impl_input_unit!(PressureUnit);
impl_input_unit!(DensityUnit);
impl_input_unit!(ViscosityUnit);

/// Persisted defaults for the per-field input units offered in Case Setup.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct InputUnitPrefs {
    pub velocity: VelocityUnit,
    pub pressure: PressureUnit,
    pub density: DensityUnit,
    pub viscosity: ViscosityUnit,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(digits: u8, notation: NumberNotation) -> ValueFormat {
        ValueFormat {
            significant_digits: digits,
            notation,
        }
    }

    #[test]
    fn display_conversions_match_exact_definitions() {
        let close = |a: f64, b: f64, tol: f64| assert!((a - b).abs() < tol, "{a} vs {b}");
        let (v, s) = display_value(Quantity::Velocity, 1.0, UnitSystem::Imperial);
        close(v, 3.280_839_895, 1e-8);
        assert_eq!(s, "ft/s");
        let (v, s) = display_value(Quantity::Pressure, 101_325.0, UnitSystem::Imperial);
        close(v, 14.695_948_775, 1e-6);
        assert_eq!(s, "psi");
        let (v, s) = display_value(Quantity::Force, 1.0, UnitSystem::Imperial);
        close(v, 0.224_808_943, 1e-8);
        assert_eq!(s, "lbf");
        let (v, s) = display_value(Quantity::Moment, 1.0, UnitSystem::Imperial);
        close(v, 0.737_562_149, 1e-8);
        assert_eq!(s, "lbf·ft");
        let (v, s) = display_value(Quantity::Density, 1.225, UnitSystem::Imperial);
        close(v, 0.076_474, 1e-5);
        assert_eq!(s, "lbm/ft³");
        let (v, s) = display_value(Quantity::Area, 1.0, UnitSystem::Imperial);
        close(v, 10.763_910_417, 1e-7);
        assert_eq!(s, "ft²");
        // SI is the identity.
        for quantity in [
            Quantity::Length,
            Quantity::Velocity,
            Quantity::Pressure,
            Quantity::Force,
            Quantity::Moment,
            Quantity::Density,
            Quantity::Viscosity,
            Quantity::Area,
        ] {
            assert_eq!(display_value(quantity, 2.5, UnitSystem::Si).0, 2.5);
        }
    }

    #[test]
    fn input_units_round_trip_through_si() {
        let close = |a: f64, b: f64| assert!((a - b).abs() < 1e-9, "{a} vs {b}");
        close(VelocityUnit::MilesPerHour.to_si(30.0), 13.4112);
        close(VelocityUnit::Knots.to_si(1.0), 0.514_444_444_444_444_4);
        close(PressureUnit::Atmosphere.to_si(1.0), 101_325.0);
        for unit in VelocityUnit::ALL {
            close(unit.from_si(unit.to_si(12.34)), 12.34);
        }
        for unit in PressureUnit::ALL {
            close(unit.from_si(unit.to_si(12.34)), 12.34);
        }
        for unit in DensityUnit::ALL {
            close(unit.from_si(unit.to_si(1.225)), 1.225);
        }
        for unit in ViscosityUnit::ALL {
            close(unit.from_si(unit.to_si(1.81e-5)), 1.81e-5);
        }
    }

    #[test]
    fn formatting_respects_significant_digits_and_notation() {
        assert_eq!(
            format_value(1.234_567, fmt(5, NumberNotation::Auto)),
            "1.2346"
        );
        assert_eq!(format_value(1234.6, fmt(3, NumberNotation::Auto)), "1235");
        assert_eq!(
            format_value(0.0012, fmt(4, NumberNotation::Auto)),
            "0.001200"
        );
        // Auto flips to scientific past its thresholds.
        assert_eq!(format_value(1.5e6, fmt(4, NumberNotation::Auto)), "1.500e6");
        assert_eq!(
            format_value(2.5e-4, fmt(3, NumberNotation::Auto)),
            "2.50e-4"
        );
        // Explicit notations override.
        assert_eq!(
            format_value(1234.5, fmt(5, NumberNotation::Scientific)),
            "1.2345e3"
        );
        assert_eq!(
            format_value(1.5e6, fmt(4, NumberNotation::Fixed)),
            "1500000"
        );
        assert_eq!(format_value(0.0, fmt(4, NumberNotation::Auto)), "0.000");
        assert_eq!(format_value(-9.876, fmt(3, NumberNotation::Auto)), "-9.88");
    }

    #[test]
    fn formatted_quantity_carries_the_unit_symbol() {
        assert_eq!(
            format_quantity(
                Quantity::Force,
                10.0,
                UnitSystem::Si,
                fmt(4, NumberNotation::Auto)
            ),
            "10.00 N"
        );
        assert_eq!(
            format_quantity(
                Quantity::Force,
                4.448_221_615_260_5,
                UnitSystem::Imperial,
                fmt(4, NumberNotation::Auto)
            ),
            "1.000 lbf"
        );
    }

    #[test]
    fn input_unit_prefs_round_trip_and_default_cleanly() {
        let prefs = InputUnitPrefs {
            velocity: VelocityUnit::MilesPerHour,
            pressure: PressureUnit::Psi,
            density: DensityUnit::PoundsMassPerCubicFoot,
            viscosity: ViscosityUnit::MillipascalSecond,
        };
        let json = serde_json::to_string(&prefs).unwrap();
        let loaded: InputUnitPrefs = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, prefs);
        let legacy: InputUnitPrefs = serde_json::from_str("{}").unwrap();
        assert_eq!(legacy, InputUnitPrefs::default());
    }
}
