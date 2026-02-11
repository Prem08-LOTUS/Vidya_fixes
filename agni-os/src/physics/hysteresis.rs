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
        // [FIX] Implemented Backlash / Directional Hysteresis
        let v = target_v.0;
        let dv = v - self.current_voltage.0;

        let nonlinear = if dv >= 0.0 {
            // Ascending Curve (Standard)
            self.alpha * v + self.beta * v.powi(2)
        } else {
            // Descending Curve (Backlash / Memory)
            // Piezo relaxes differently. We subtract a hysteresis term.
            // Simplified Preisach: Effective voltage is slightly higher than applied.
            let v_eff = v + 0.5; // Backlash width
            self.alpha * v_eff + self.beta * v_eff.powi(2)
        };

        self.current_voltage = target_v;
        Nanometers(nonlinear)
    }
}
