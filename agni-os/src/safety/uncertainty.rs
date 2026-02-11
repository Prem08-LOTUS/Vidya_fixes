// agni-workspace/agni-os/src/safety/uncertainty.rs
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Measurement<T> {
    pub value: T,
    pub uncertainty_1sigma: f64,
    pub timestamp_cycle: u64,
}

impl<T> Measurement<T> {
    pub fn new(value: T, sigma: f64, cycle: u64) -> Self {
        Self { value, uncertainty_1sigma: sigma, timestamp_cycle: cycle }
    }

    /// EPISTEMIC GATE
    /// Returns Ok only if uncertainty is known, finite, and within limits.
    pub fn trust_or_fail(&self, limit: f64) -> Result<&T, String> {
        // A-13: NaN check.
        // NaN comparisons (NaN > limit) return false, which would unsafely pass the check
        // if we didn't explicitly guard against it.
        if self.uncertainty_1sigma.is_nan() {
            return Err("EPISTEMIC FAILURE: Uncertainty is NaN".to_string());
        }

        if self.uncertainty_1sigma < 0.0 {
             return Err("EPISTEMIC FAILURE: Negative Uncertainty".to_string());
        }

        if self.uncertainty_1sigma > limit {
            Err(format!(
                "EPISTEMIC FAILURE: Uncertainty {:.4} > Limit {:.4}",
                self.uncertainty_1sigma, limit
            ))
        } else {
            Ok(&self.value)
        }
    }
}

/// Evolves uncertainty over time (Kalman Time Update).
///
/// Invariant: Uncertainty MUST NOT decrease without measurement.
/// sigma_new = sqrt(sigma_old^2 + Q * dt)
pub fn evolve_uncertainty(sigma_old: f64, dt: f64, process_noise: f64) -> f64 {
    // Fail safe on NaN or Non-Finite inputs (Fail-Closed)
    if !sigma_old.is_finite() || !dt.is_finite() || !process_noise.is_finite() {
        return f64::INFINITY;
    }

    // Standard Kalman Time Update: sigma_new^2 = sigma_old^2 + Q*dt
    // We strictly assume Q > 0 and dt >= 0.
    // If dt < 0 (clock skew), we clamp to 0.
    let dt_clamped = dt.max(0.0);

    // Prevent Inf * 0.0 => NaN if Q is infinite (though we rejected infinite Q above)
    // and optimize no-op
    if dt_clamped == 0.0 {
        return sigma_old;
    }

    let q = process_noise.abs(); // Q is variance, must be positive

    let variance_old = sigma_old.powi(2);

    let variance_new = variance_old + q * dt_clamped;

    // Clamp to sigma_old to prevent underflow-induced entropy violation
    variance_new.sqrt().max(sigma_old)
}
