use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::physics::types::{Nanometers, ThermalField, Voltage};
use crate::safety::uncertainty::Measurement;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DataPoint {
    pub z_nm: f64,
    pub timestamp_ms: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SystemStatus {
    pub safe: bool,
    pub rh: f64,
    pub temperature: f64,
    pub thermal_field: Option<ThermalField>,
    pub thermal_gradient: f64,
    pub z_nm: f64,
    pub z_sigma: f64,
    pub input_voltage: f64, // [NEW] Brownout monitoring
    pub sensor_health: String, // [NEW] Handshake/Connection status
    pub timestamp: String,
    pub last_error: Option<String>,

    // Burst Buffer
    pub history: Vec<DataPoint>,
}

impl Default for SystemStatus {
    fn default() -> Self {
        Self {
            safe: false,
            rh: 0.0,
            temperature: 0.0,
            thermal_field: None,
            thermal_gradient: 0.0,
            z_nm: 0.0,
            z_sigma: 0.0,
            input_voltage: 0.0,
            sensor_health: "UNKNOWN".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            last_error: None,
            history: Vec::new(),
        }
    }
}

// --- NEW PROTOCOL ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlCommand {
    MoveZ { target: Nanometers, intent_id: String },
    ScanPath {
        points: Vec<(f64, f64)>,
        velocity_um_s: f64,
        intent_id: String,
    },
    Halt,
    Resume,
}

#[derive(Debug, Clone)]
pub enum TelemetryEvent {
    Status {
        timestamp: DateTime<Utc>,
        z_pos: Measurement<Nanometers>,
        safe: bool,
        thermal_field: Measurement<ThermalField>,
        control_effort: Voltage,
        input_voltage: f64, // [NEW]
        sensor_health: String, // [NEW]
    },
    MotionComplete { intent_id: String },
    MotionError { intent_id: String, error: String },
}
