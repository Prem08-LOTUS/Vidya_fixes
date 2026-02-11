#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]

pub mod database;
pub mod motion_controller;
pub mod gdsii_processor;
pub mod compute;
pub mod sensor_monitor;
pub mod types;
pub mod supervisor;
pub mod persistence;
pub mod safety;
pub mod physics;
pub mod metrology;
pub mod io;
pub mod telemetry;
pub mod commissioning;
pub mod config;
pub mod verification_harness; // [NEW] Expose verification

pub use compute::{ComputeEngine, PiezoKalman, MPC};
pub use types::SystemStatus;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
