// agni-workspace/agni-os/src/compute/kalman.rs
use nalgebra::{Matrix2, Vector2};

#[derive(Debug, Clone)]
pub struct PiezoKalman {
    state: Vector2<f64>, // [pos, vel]
    cov: Matrix2<f64>,
    // Config
    gate_threshold_sigma: f64, // e.g. 5.0
}

impl PiezoKalman {
    pub fn new() -> Self {
        Self {
            state: Vector2::zeros(),
            cov: Matrix2::identity(),
            gate_threshold_sigma: 5.0, // Reject anything > 5 sigma
        }
    }

    /// Update with Innovation Gating
    /// Returns: Ok(EstimatedPos) or Err(RejectionReason)
    pub fn update_checked(&mut self, meas: f64, dt: f64) -> Result<f64, &'static str> {
        // 1. Epistemic Sanity: Reject NaN/Inf Inputs immediately
        if !meas.is_finite() || !dt.is_finite() || dt <= 0.0 {
            return Err("KALMAN_POISON: Input is NaN/Inf or Invalid DT");
        }

        // Prediction Step (Simplified Physics)
        let f = Matrix2::new(1.0, dt, 0.0, 1.0);
        let pred_state = f * self.state;
        let q = Matrix2::identity() * 0.1; // Process noise
        let pred_cov = f * self.cov * f.transpose() + q;

        // 2. Innovation Gating (The "Noise" Fix)
        // Calculate Innovation (Residual)
        let h = Vector2::new(1.0, 0.0).transpose(); // We measure position
        let residual = meas - (h * pred_state)[0];

        // Innovation Covariance S = HPH' + R
        let r = 0.5; // Measurement noise
        let s = (h * pred_cov * h.transpose())[0] + r;

        // Mahalanobis Distance (Normalized Error)
        let sigma_dist = residual.abs() / s.sqrt();

        if sigma_dist > self.gate_threshold_sigma {
            // Outlier detected! Reject measurement, keep prediction.
            // This prevents the "66um jump".
            self.state = pred_state;
            self.cov = pred_cov;
            return Err("KALMAN_GATED: Measurement rejected (> 5 sigma jump)");
        }

        // Update Step
        let k = pred_cov * h.transpose() / s;
        let new_state = pred_state + k * residual;
        let i = Matrix2::identity();
        let new_cov = (i - k * h) * pred_cov;

        // 3. Post-Process Sanity
        if !new_state[0].is_finite() {
            return Err("KALMAN_CORRUPTION: State became NaN after update");
        }

        self.state = new_state;
        self.cov = new_cov;

        Ok(self.state[0])
    }
}
