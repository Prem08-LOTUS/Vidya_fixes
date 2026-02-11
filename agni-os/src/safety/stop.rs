/// THE FAIL-SAFE CONTRACT
/// Every subsystem must implement this.
pub trait FailSafe {
    /// Returns true ONLY if the subsystem has valid, fresh, safe data.
    fn is_healthy(&self) -> bool;

    /// Immediately put hardware in a safe state.
    /// MUST NOT BLOCK. MUST NOT ALLOCATE.
    fn emergency_stop(&mut self);
}

/// THE CASCADE
/// If one part dies, everything dies.
pub fn assert_system_health(subsystems: &mut [Box<dyn FailSafe>]) {
    let mut compromised = false;
    for sys in subsystems.iter() {
        if !sys.is_healthy() {
            compromised = true;
            break;
        }
    }
    if compromised {
        for sys in subsystems.iter_mut() {
            sys.emergency_stop();
        }
        // In real HW we might trap/abort here.
        tracing::error!("SYSTEM HEALTH ASSERTION FAILED. CASCADE STOP.");
    }
}
