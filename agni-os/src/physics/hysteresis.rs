use crate::physics::types::{Voltage, Nanometers};

/// ASSUMPTION 32: Piezo is memoryless (FALSE).
/// This struct implements the memory of the material (Preisach-ish).
pub struct HysteresisState {
    current_voltage: Voltage,
    alpha: f64,
    beta: f64,
}

impl HysteresisState {
    pub fn new() -> Self {
        Self {
            current_voltage: Voltage(0.0),
            alpha: 0.1,
            beta: 0.05,
        }
    }

    /// CONTRACT: PATH DEPENDENCE
    /// Output depends on Input AND Internal State.
    pub fn predict_displacement(&mut self, target_v: Voltage) -> Nanometers {
        // Simple quadratic model for now, replacing the "Scalar Assumption"
        let v = target_v.0;
        let nonlinear = self.alpha * v + self.beta * v.powi(2);
        self.current_voltage = target_v;
        Nanometers(nonlinear)
    }
}
