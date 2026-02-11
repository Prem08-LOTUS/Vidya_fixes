use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::RwLock;
use crate::physics::types::ThermalField;

#[derive(Debug)]
pub struct SafetyState {
    pub metrology_ok: AtomicBool,
    pub power_ok: AtomicBool,
    pub watchdog_ok: AtomicBool,
    pub last_update_ns: AtomicU64,

    // Complex thermal field requires lock
    pub thermal_field: RwLock<ThermalField>,
    pub rh_bits: AtomicU64,
}

impl SafetyState {
    pub fn new() -> Self {
        Self {
            metrology_ok: AtomicBool::new(false),
            power_ok: AtomicBool::new(true),
            watchdog_ok: AtomicBool::new(true),
            last_update_ns: AtomicU64::new(0),
            thermal_field: RwLock::new(ThermalField::default()),
            rh_bits: AtomicU64::new(0),
        }
    }

    pub fn is_safe(&self) -> bool {
        // Use Acquire/Release for synchronization instead of Relaxed
        self.metrology_ok.load(Ordering::Acquire)
            && self.power_ok.load(Ordering::Acquire)
            && self.watchdog_ok.load(Ordering::Acquire)
    }

    pub fn set_metrology_data(&self, safe: bool, field: ThermalField, rh: f64) {
        self.metrology_ok.store(safe, Ordering::Release);
        if let Ok(mut w) = self.thermal_field.write() {
            *w = field;
        }
        self.rh_bits.store(rh.to_bits(), Ordering::Release);
    }

    pub fn get_metrology_data(&self) -> (f64, f64) {
        // Compatibility: Return base temp and RH
        let t = if let Ok(r) = self.thermal_field.read() {
            r.sensors[0] // Use first sensor as scalar temp
        } else {
            0.0
        };
        let r = f64::from_bits(self.rh_bits.load(Ordering::Acquire));
        (t, r)
    }

    pub fn get_thermal_field(&self) -> ThermalField {
        if let Ok(r) = self.thermal_field.read() {
            *r
        } else {
            ThermalField::default()
        }
    }
}
