/// Control Barrier Function (Hard Deck) Logic
///
/// Ensures the system state never leaves the safe set defined by braking physics.
/// h(x) = distance_remaining - v^2 / 2a >= 0

/// Validates that a requested velocity command allows stopping before the limit.
///
/// Returns `Ok(cmd_vel)` if safe.
/// Returns `Err("HARD_DECK_VIOLATION")` or specific error if unsafe.
///
/// FAIL-STOP: This function does not clamp. If safety is violated, it returns an error
/// forcing the caller to halt or take emergency action.
pub fn validate_hard_deck(
    pos: f64,
    limit: f64,
    max_accel: f64,
    cmd_vel: f64,
) -> Result<f64, &'static str> {

    // 1. Sanity Checks
    if max_accel <= 0.0 || !max_accel.is_finite() {
        return Err("CONFIG_ERROR"); // Configuration error -> Halt
    }
    if pos.is_nan() || limit.is_nan() || cmd_vel.is_nan() {
        return Err("SENSOR_FAULT"); // Sensor error -> Halt
    }

    let dist_remaining = (limit - pos).abs();

    // FIX: Infinite distance implies sensor failure or configuration error in this context
    if dist_remaining.is_infinite() {
        return Err("SENSOR_FAULT_INF");
    }

    // v^2 / 2a
    let v_sq = cmd_vel.powi(2);

    // FIX: Check for overflow
    if v_sq.is_infinite() {
        return Err("PHYSICS_OVERFLOW");
    }

    let braking_dist_needed = v_sq / (2.0 * max_accel);

    // Safety margin (10%)
    let margin = (dist_remaining * 0.1).max(1.0);

    if braking_dist_needed < (dist_remaining - margin) {
        Ok(cmd_vel)
    } else {
        // Violation! FAIL-STOP.
        Err("HARD_DECK_VIOLATION")
    }
}
