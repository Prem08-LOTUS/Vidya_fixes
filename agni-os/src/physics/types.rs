use serde::{Serialize, Deserialize};
use nalgebra::{Vector3, Matrix3};

// --- STRONG TYPES (Assumption 31, 74) ---
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Nanometers(pub f64);

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Voltage(pub f64);

// Trait Impls for Generic Voting (T -> f64 -> T)
// Rule S-VOTE-01 Compliance: These are linear, lossless wrappers.
impl From<f64> for Nanometers {
    fn from(val: f64) -> Self { Nanometers(val) }
}
impl Into<f64> for Nanometers {
    fn into(self) -> f64 { self.0 }
}

impl From<f64> for Voltage {
    fn from(val: f64) -> Self { Voltage(val) }
}
impl Into<f64> for Voltage {
    fn into(self) -> f64 { self.0 }
}

// --- THERMAL MANIFOLD (Assumption 1, 3, 6) ---
// Temperature is not a scalar. It is a vector field with velocity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThermalField {
    // Spatial State
    pub sensors: [f64; 4],         // Raw readings
    pub gradient: Vector3<f64>,    // Spatial derivative (dT/dx, dT/dy, dT/dz)

    // Temporal State (Thermal Velocity)
    pub d_gradient_dt: Vector3<f64>,

    // Epistemic State
    pub covariance: Matrix3<f64>,  // Uncertainty correlation
    pub timestamp_cycle: u64,
}

impl Default for ThermalField {
    fn default() -> Self {
        Self {
            sensors: [0.0; 4],
            gradient: Vector3::zeros(),
            d_gradient_dt: Vector3::zeros(),
            covariance: Matrix3::identity(),
            timestamp_cycle: 0,
        }
    }
}

impl ThermalField {
    /// CONTRACT: STABILITY
    /// Stable means low gradient AND low thermal velocity.
    pub fn is_stable(&self, grad_limit: f64, velocity_limit: f64) -> bool {
        let g_norm = self.gradient.norm();
        let v_norm = self.d_gradient_dt.norm();
        g_norm < grad_limit && v_norm < velocity_limit
    }
}
