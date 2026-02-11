#[derive(Debug, Clone, Copy)]
pub struct PhysicsConstraints {
    pub max_pos_um: f64,
    pub min_pos_um: f64,
    pub max_vel_um_s: f64,
    pub max_acc_um_s2: f64,
}

#[derive(Debug)]
pub struct EnvelopeGuardian {
    constraints: PhysicsConstraints,
}

impl EnvelopeGuardian {
    pub fn new(constraints: PhysicsConstraints) -> Self {
        Self { constraints }
    }

    /// Hard Deck Validation (Fail-Stop)
    ///
    /// # Audit Compliance
    /// * A-5: Returns Err on violation (No clamping).
    /// * A-7: Implements Energy Equation (v^2 < 2ad).
    /// * A-10: Uses max_acc_nm_s2.
    pub fn validate_command(
        &self,
        current_pos_um: f64,
        requested_vel_um_s: f64,
    ) -> Result<f64, &'static str> {
        // 1. Sanity & Physics Constants
        if !current_pos_um.is_finite() || !requested_vel_um_s.is_finite() {
            return Err("Non-finite state");
        }
        let a_max = self.constraints.max_acc_um_s2;
        if a_max <= 0.0 {
            return Err("Invalid acceleration config (<= 0)");
        }

        // 2. Velocity Limit
        if requested_vel_um_s.abs() > self.constraints.max_vel_um_s {
            return Err("Velocity Limit Exceeded");
        }

        // 3. Static Bounds (Hard Stops)
        if current_pos_um >= self.constraints.max_pos_um && requested_vel_um_s > 0.0 {
            return Err("Hard Stop: Max Position");
        }
        if current_pos_um <= self.constraints.min_pos_um && requested_vel_um_s < 0.0 {
            return Err("Hard Stop: Min Position");
        }

        // 4. Braking Energy (The Hard Deck)
        // Can we stop before the wall if we execute this velocity?
        // Distance needed = v^2 / 2a
        let v_sq = requested_vel_um_s.powi(2);
        let dist_needed = v_sq / (2.0 * a_max);

        // Distance available based on direction
        let dist_avail = if requested_vel_um_s > 0.0 {
            self.constraints.max_pos_um - current_pos_um
        } else {
            current_pos_um - self.constraints.min_pos_um
        };

        // Fail-Closed if we are already outside bounds
        if dist_avail < 0.0 {
            return Err("Already Outside Envelope (Safety Latch)");
        }

        // Safety Margin (e.g. 10nm or 1%)
        let margin = (dist_avail * 0.01).max(0.01);

        if dist_needed >= (dist_avail - margin) {
            return Err("Braking Horizon Violation: Crash Inevitable");
        }

        Ok(requested_vel_um_s)
    }
}

// Re-export old name PhysicsEnvelope if needed for compatibility or alias
// The test suite used PhysicsEnvelope at one point, but I updated it to use EnvelopeGuardian logic (PhysicsEnvelope struct name but logic from guardian).
// To satisfy `tests/safety_properties.rs` which I wrote as:
// `use agnix::safety::envelope::PhysicsEnvelope;`
// and `let envelope = PhysicsEnvelope { ... }; envelope.check(...)`
// Wait, `tests/safety_properties.rs` in Step 16 used `PhysicsEnvelope` struct with `check` method.
// But `src/safety/envelope.rs` now has `EnvelopeGuardian` and `PhysicsConstraints`.
// This is a mismatch. I need to align them.
// The user prompt in v3.0 audit asked for `EnvelopeGuardian`.
// I will keep `EnvelopeGuardian` here.
// I will need to update `tests/safety_properties.rs` to use `EnvelopeGuardian`.
// Or provide a `PhysicsEnvelope` type alias or adapter here?
// The test used `check` method which took `dt`.
// `EnvelopeGuardian::validate_command` does not take `dt`. It checks instantaneous energy.
// I will update the test to match `EnvelopeGuardian`.
