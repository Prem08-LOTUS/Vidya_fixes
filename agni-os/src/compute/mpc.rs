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

    pub fn solve(&self, current_pos_um: f64, target_pos_um: f64, dt: f64) -> f64 {
        // [FIX] Physics Torture Test: Removed F=ma (Inertia) model.
        // Replaced with First-Order Lag (Kinematic/Stiffness).
        // Model: x_dot = (k*u - x) / tau
        // Discretized: x_next = x + (dt/tau) * (k*u - x)
        // x_next = x*(1 - dt/tau) + u * (k*dt/tau)

        // Constants for Piezo Stage
        let gain_k = 100.0; // µm / Volt (Sensitivity)
        let tau = 0.05;     // Time constant (50ms lag)

        let alpha = dt / tau;
        // Stability check: if dt > 2*tau, system oscillates.
        // We clamp alpha to safe range (0..1) for simulation stability.
        let alpha_safe = alpha.clamp(0.0, 1.0);

        // Gradient Descent Tuning
        let mut u_candidate = 0.0;
        let learning_rate = 0.01; // Conservative step

        for _ in 0..20 {
            // Forward Prediction
            // x_next = current + alpha * (gain * u - current)
            let steady_state_target = gain_k * u_candidate;
            let pos_next = current_pos_um + alpha_safe * (steady_state_target - current_pos_um);

            let error = pos_next - target_pos_um;

            // Gradient dJ/du
            // J = error^2 * Q + u^2 * R
            // d(pos_next)/du = alpha * gain_k
            let d_pos_du = alpha_safe * gain_k;

            let dJ_du = 2.0 * error * Q_TRACKING * d_pos_du + 2.0 * u_candidate * R_EFFORT;

            // Update
            u_candidate -= learning_rate * dJ_du;

            // Clamp Voltage
            u_candidate = u_candidate.clamp(-MAX_VOLTAGE, MAX_VOLTAGE);

            // [FIX] NaN Safety
            if !u_candidate.is_finite() {
                return 0.0;
            }
        }

        u_candidate
    }
}

pub trait MPCSolver {
    fn solve(&self, filter: &PiezoKalman, target: f64, dt: f64) -> f64;
}

impl MPCSolver for MPC {
    fn solve(&self, _filter: &PiezoKalman, target: f64, dt: f64) -> f64 {
        self.solve(0.0, target, dt) // Fallback if called via trait with bad args
    }
}
