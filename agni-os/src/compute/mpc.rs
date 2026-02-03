use nalgebra::{Vector2, Matrix2};
use super::kalman::PiezoKalman;

// TUNING PARAMETERS (ADJUSTED FOR AGGRESSIVE TRACKING)
const N_HORIZON: usize = 10;
const Q_TRACKING: f64 = 100.0; // Increased from 1.0 (Prioritize reaching target)
const R_EFFORT: f64 = 0.1;     // Decreased from 1.0 (Cheap to use voltage)
const MAX_VOLTAGE: f64 = 10.0; // 10V limit

#[derive(Debug, Clone)]
pub struct MPC {
    // Legacy fields kept for compatibility if needed, or simplified
    state: Vector2<f64>, // [position, velocity]
}

impl MPC {
    pub fn new(_horizon: usize, _num_candidates: usize) -> Self {
        Self { state: Vector2::zeros() }
    }

    /// Solves the optimal control problem for the next step.
    /// Returns the voltage (u) to apply.
    /// Replaces solve_optimal.
    pub fn solve_optimal(&self, _filter: &PiezoKalman, target_pos_um: f64) -> f64 {
        self.solve(0.0, target_pos_um) // Temporary, assumes 0.0 start
    }

    pub fn solve(&self, current_pos_um: f64, target_pos_um: f64) -> f64 {
        let dt = 0.01; // 10ms

        // SIMPLE GRADIENT DESCENT SOLVER
        // Cost J = sum( (pos - target)^2 * Q + u^2 * R )
        // We iterate to find u that minimizes J.

        let mut u_candidate = 0.0;
        let learning_rate = 0.5;

        for _ in 0..20 { // 20 iterations
            // Predicted next state based on u_candidate
            // x_next = x + v*dt
            // v_next = v + a*dt = v + (u * coupling)*dt
            // Simple model: 1V = 10 um/s^2 (Assumed coupling)
            let coupling = 10.0;

            // Prediction (One step lookahead for simplicity in this hotfix)
            // In a full MPC, we'd unroll N_HORIZON.
            // Here we fix the "non-positive" bug by ensuring the gradient points towards the target.

            let pos_next = current_pos_um + 0.0 * dt + 0.5 * (u_candidate * coupling) * dt * dt;
            let error = pos_next - target_pos_um;

            // Gradient dJ/du
            // J = error^2 * Q + u^2 * R
            // dJ/du = 2*error * (d_error/du) + 2*u*R
            // d_error/du = 0.5 * coupling * dt^2

            let d_error_du = 0.5 * coupling * dt * dt;
            let gradient = 2.0 * error * Q_TRACKING * d_error_du + 2.0 * u_candidate * R_EFFORT;

            // Descent
            u_candidate -= learning_rate * gradient;

            // Clamp
            u_candidate = u_candidate.clamp(-MAX_VOLTAGE, MAX_VOLTAGE);
        }

        u_candidate
    }
}

pub trait MPCSolver {
    fn solve(&self, filter: &PiezoKalman, target: f64) -> f64;
}

impl MPCSolver for MPC {
    fn solve(&self, _filter: &PiezoKalman, target: f64) -> f64 {
        self.solve_optimal(_filter, target)
    }
}
