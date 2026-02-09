//! Compute Engine: Kalman Filter + MPC
//!
//! Purpose: Omniscient decision making for Z-axis motion
//!
//! Architecture:
//! 1. Sensor Monitor reads FDC1004 (noisy)
//! 2. Kalman Filter estimates true position + velocity
//! 3. MPC solver finds optimal voltage to reach target
//! 4. Motion Controller applies voltage and repeats
//!
//! This runs 100× per second on Raspberry Pi 4

pub mod kalman;
pub mod mpc;

pub use kalman::PiezoKalman; // [UPDATED] Expose the hardened Kalman
pub use mpc::{MPC, MPCSolver};

/// Complete compute engine state
#[derive(Debug, Clone)]
pub struct ComputeEngine {
    // filter removed, managed by MotionController
    pub mpc: MPC,
}

impl ComputeEngine {
    /// Create new compute engine
    pub fn new(_initial_z: f64, _dt: f64) -> Self {
        Self {
            mpc: MPC::new(10, 201),  // 10-step horizon, 201 candidates
        }
    }

    /// One control loop iteration
    ///
    /// # Arguments
    /// * `filtered_z` - Kalman estimate from MotionController (µm)
    /// * `target_z` - Desired position (µm)
    ///
    /// # Returns
    /// Optimal voltage command (V)
    pub fn step(&mut self, filtered_z: f64, target_z: f64, dt: f64) -> f64 {
        // [FIX] Actually call MPC
        self.mpc.solve(filtered_z, target_z, dt)
    }

    pub fn position(&self) -> f64 {
        0.0 // Placeholder
    }
}
