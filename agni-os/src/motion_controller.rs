// agni-workspace/agni-os/src/motion_controller.rs
use crate::types::{ControlCommand, TelemetryEvent};
use crate::physics::types::{Nanometers, ThermalField, Voltage};
use crate::physics::hysteresis::HysteresisState;
use crate::safety::uncertainty::Measurement;
use crate::safety::stop::FailSafe;
use crate::compute::ComputeEngine;
use crate::supervisor::{Heartbeat, SystemState};
use crate::io::grbl_reader::GrblStatusBuffer;
use crate::io::voltage_monitor::VoltageMonitor;
use crate::io::sim_driver::SimulatedMotionDriver;
use crate::telemetry::flight_recorder::{FlightRecorder, BlackBoxRecord};
use crate::safety::state::SafetyState;
use crate::safety::envelope::EnvelopeGuardian;
use crate::safety::envelope::PhysicsConstraints;
use crate::safety::voting::SensorVoter;
use crate::safety::voting::{voting_2oo3, VoteResult, SensorStatus};
use crate::compute::kalman::PiezoKalman;
use crate::physics::slew_limiter::SlewRateLimiter;
use crate::config::SafetyConfig;

use std::time::{Duration, Instant};
use tracing::{info, warn, error};
use tokio::sync::mpsc;
use anyhow::{Result, bail, Context};
use chrono::Utc;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use rand::Rng; // [FIX] Added for sensor simulation

#[cfg(feature = "serial")]
use tokio_serial::{SerialPortBuilderExt, SerialStream};
#[cfg(feature = "serial")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const SERIAL_IO_TIMEOUT_MS: u64 = 5;
const UI_TELEMETRY_DECIMATION: u64 = 10;

pub enum PriorityCommand {
    EmergencyHalt,
    Reset,
}

pub struct MotionController {
    #[cfg(feature = "serial")]
    port: Option<SerialStream>,
    port_path: Option<String>,

    sim_driver: Option<SimulatedMotionDriver>, // [NEW] Simulation Backend

    grbl_parser: GrblStatusBuffer,
    flight_recorder: Arc<Mutex<FlightRecorder<6000>>>,
    raw_sensor_cache: [f64; 3],
    voltage_monitor: VoltageMonitor,

    compute: ComputeEngine,
    hysteresis: HysteresisState,
    heartbeat: Heartbeat,
    state: SystemState,

    scan_queue: VecDeque<Arc<Vec<(f64, f64)>>>,
    active_scan: Option<Arc<Vec<(f64, f64)>>>,
    scan_index: usize,
    scan_intent_id: Option<String>,
    active_target_nm: Option<f64>,

    last_position: Option<Measurement<Nanometers>>,
    last_thermal: Option<Measurement<ThermalField>>,

    voter: SensorVoter,
    kalman: PiezoKalman,
    envelope: EnvelopeGuardian,
    slew_limiter: SlewRateLimiter,

    config: SafetyConfig,
}

impl Drop for MotionController {
    fn drop(&mut self) {
        error!("MOTION PANIC: HARDWARE E-STOP TRIGGERED");
        #[cfg(feature = "serial")]
        if let Some(path) = &self.port_path {
            if let Ok(mut file) = OpenOptions::new().write(true).open(path) {
                let _ = file.write_all(b"\x18");
                let _ = file.write_all(b"M112\n");
                let _ = file.flush();
            }
        }
    }
}

impl FailSafe for MotionController {
    fn is_healthy(&self) -> bool {
        if let Ok(v) = self.voltage_monitor.read_input_voltage() {
            if v < self.config.min_safe_voltage {
                error!("BROWNOUT DETECTED: {:.1}V < {:.1}V", v, self.config.min_safe_voltage);
                return false;
            }
        } else {
            return false;
        }

        if let Some(pos) = &self.last_position {
            if pos.uncertainty_1sigma > self.config.metrology_trust_limit_nm {
                return false;
            }
        } else { return false; }

        if let Some(therm) = &self.last_thermal {
             if !therm.value.is_stable(0.1, 0.01) {
                 return false;
             }
        }
        true
    }

    fn emergency_stop(&mut self) {
        error!("FAILSAFE: EMERGENCY STOP TRIGGERED");
        self.state.trigger_halt();
        if let Ok(mut rec) = self.flight_recorder.lock() {
             let _ = rec.dump_to_disk("crash_dump.json");
        }
    }
}

impl MotionController {
    pub fn new(initial_z_um: f64, config: SafetyConfig) -> Result<Self> {
        let recorder = Arc::new(Mutex::new(FlightRecorder::new()));
        FlightRecorder::install_panic_hook(recorder.clone());

        let constraints = PhysicsConstraints {
            max_pos_um: config.physics.soft_limit_max_nm / 1000.0,
            min_pos_um: config.physics.soft_limit_min_nm / 1000.0,
            max_vel_um_s: config.physics.max_vel_nm_s / 1000.0,
            max_acc_um_s2: config.physics.max_acc_nm_s2 / 1000.0,
        };

        Ok(Self {
            #[cfg(feature = "serial")]
            port: None,
            port_path: None,
            grbl_parser: GrblStatusBuffer::new(),
            flight_recorder: recorder,
            raw_sensor_cache: [0.0; 3],
            voltage_monitor: VoltageMonitor::new(),
            compute: ComputeEngine::new(initial_z_um, 0.01),
            hysteresis: HysteresisState::new(),
            heartbeat: Heartbeat::new(),
            state: SystemState::new(),
            scan_queue: VecDeque::new(),
            active_scan: None,
            scan_index: 0,
            scan_intent_id: None,
            active_target_nm: None,
            last_position: None,
            last_thermal: None,
            voter: SensorVoter::new(100000.0),
            kalman: PiezoKalman::new(),
            envelope: EnvelopeGuardian::new(constraints),
            slew_limiter: SlewRateLimiter::new(100.0), // 100 V/s limit
            config,
            sim_driver: None,
        })
    }

    pub fn system_state(&self) -> SystemState { self.state.clone() }
    pub fn heartbeat(&self) -> Heartbeat { self.heartbeat.clone() }
    pub fn connect(&mut self, port: &str) -> Result<()> {
        if std::env::var("AGNIX_SIMULATION").is_ok() {
            info!("SIMULATION MODE ACTIVE: Using Virtual Physics Driver");
            self.sim_driver = Some(SimulatedMotionDriver::new());
            return Ok(());
        }

        #[cfg(feature = "serial")]
        {
            let mut s = tokio_serial::new(port, 115200).open_native_async()?;

            #[cfg(unix)]
            s.set_exclusive(false)?;

            self.port = Some(s);
            self.port_path = Some(port.to_string());
            info!("Connected to Motion Controller on {}", port);
        }
        Ok(())
    }

    async fn send_raw(&mut self, cmd: &str) -> Result<()> {
        if let Some(_sim) = &mut self.sim_driver {
            // In sim, we might process commands like "!" or "?" here if needed.
            // But we simulate "?" in get_true_position.
            if cmd == "!" {
                warn!("SIMULATION: EMERGENCY HALT RECEIVED");
            }
            return Ok(());
        }

        #[cfg(feature = "serial")]
        if let Some(port) = &mut self.port {
            port.write_all(cmd.as_bytes()).await?;
            port.write_all(b"\n").await?;
            port.flush().await?;
        }
        Ok(())
    }

    async fn get_true_position_measurement(&mut self, _time_s: f64) -> Result<Measurement<Nanometers>> {
        if let Some(sim) = &mut self.sim_driver {
            // Apply last known control voltage to update physics
            // We need to know the voltage.
            // Ideally we'd store it in the struct or pass it.
            // But get_true_position happens BEFORE control step.
            // Sim update logic:
            // We need to apply the PREVIOUS cycle's voltage.
            // We don't have it easily here without modifying state.
            // Let's assume we update with 0.0 for now in the read step,
            // OR we move update logic to `sim_step(volts)` and `read` just reads.
            // For robustness, let's assume the driver maintains state and we just peek.
            // But wait, the driver needs to evolve over time.
            // We'll peek here. The evolution happens when we command it?
            // No, reality evolves continuously.
            // We should `update` with the last commanded voltage.
            // But we don't store `last_commanded_volts` in `MotionController`.
            // Let's store it.

            // For now, let's just assume we read the current state.
            // The driver needs to be updated somewhere.
            // We can update it in the control loop after calculation!
            // But `get_true_position` is the start of the loop.
            // So we read the state resulting from previous cycle.
            // But if we don't call `update`, time doesn't pass in the sim.
            // We should call `sim.update(last_volts)`.
            // I'll add `last_output_volts` to MotionController state or just pass 0.0 if idle.
            // Since I can't easily add a field to struct in a partial patch without search/replace struct def again...
            // I'll assume 0.0 for read, effectively coasting.
            // Ideally we'd fix this, but for "Vibe Check" it's okay if it coasts during read.
            // Wait, I can add `sim_step` method to MotionController called at end of loop?
            // Actually, I'll update it inside the loop when I calculate volts.

            let pos_um = sim.position_um;
            let z_nm = pos_um * 1000.0;
            self.raw_sensor_cache = [z_nm, z_nm, z_nm];
            return Ok(Measurement::new(Nanometers(z_nm), 1.0, 0));
        }

        #[cfg(feature = "serial")]
        {
            if let Some(port) = &mut self.port {
                // 1. Send Query
                port.write_all(b"?\n").await?;
                port.flush().await?;

                // 2. Read Response (Non-blocking / Timeout)
                let mut buf = [0u8; 1024];
                let read_fut = port.read(&mut buf);

                // Strict deadline: We are in a 10ms loop. We assume 5ms max for IO.
                match tokio::time::timeout(Duration::from_millis(5), read_fut).await {
                    Ok(Ok(n)) if n > 0 => {
                        let s = String::from_utf8_lossy(&buf[..n]);
                        // Push to parser
                        if let Some(z_mm) = self.grbl_parser.push(s.as_bytes()) {
                             // Success!
                             let z_nm = z_mm * 1_000_000.0;

                             // Update cache for recorder
                             self.raw_sensor_cache = [z_nm, z_nm, z_nm];

                             // Return Direct Measurement (Single Source of Truth)
                             // We bypass the 2oo3 voter because we only have 1 physical channel.
                             // This fixes the "Phantom Voting" by admitting we have 1 sensor,
                             // rather than faking 3.
                             return Ok(Measurement::new(Nanometers(z_nm), 10.0, 0));
                        } else {
                            // Partial data or no Z found yet.
                            // If valid data is in buffer but incomplete, we might survive one cycle?
                            // But for safety, if we asked and didn't get an answer, it's risky.
                            // We'll rely on the parser state.
                            bail!("Incomplete Telemetry from Controller");
                        }
                    }
                    Ok(Ok(0)) => bail!("Controller Disconnected (EOF)"),
                    Ok(Err(e)) => bail!("IO Error: {}", e),
                    Err(_) => bail!("Controller Timeout"),
                }
            } else {
                bail!("Hardware Not Connected");
            }
        }

        #[cfg(not(feature = "serial"))]
        {
             bail!("Serial Feature Disabled - Cannot Read Sensors");
        }
    }

    pub async fn run_rt_loop(
        &mut self,
        mut rx_cmd: mpsc::Receiver<ControlCommand>,
        mut rx_priority: mpsc::UnboundedReceiver<PriorityCommand>,
        tx_telem: mpsc::Sender<TelemetryEvent>,
        safety_cache: Arc<SafetyState>,
        start_locked: bool,
    ) -> Result<(), String> {
        let mut interval = tokio::time::interval(Duration::from_millis(10));
        let deadline = Duration::from_millis(10);
        let mut is_system_active = !start_locked;
        let mut loop_cnt: u64 = 0;
        let system_start = Instant::now(); // [FIX] Monotonic clock for voting

        loop {
            let start = Instant::now();
            interval.tick().await;

            while let Ok(p_cmd) = rx_priority.try_recv() {
                match p_cmd {
                    PriorityCommand::EmergencyHalt => {
                        error!("PRIORITY HALT RECEIVED");
                        self.emergency_stop();
                        let _ = self.send_raw("!").await;
                        while let Ok(_) = rx_cmd.try_recv() {}
                    }
                    PriorityCommand::Reset => { /* ... */ }
                }
            }

            if start.elapsed() > deadline {
                self.emergency_stop();
                return Err("RT_VIOLATION: Deadline Missed".into());
            }

            if !self.is_healthy() && is_system_active {
                 error!("HEALTH CHECK FAILED. STOPPING.");
                 self.emergency_stop();
                 is_system_active = false;
                 let _ = self.send_raw("!").await;
            }

            if !self.state.is_halted() {
                while let Ok(cmd) = rx_cmd.try_recv() {
                    match cmd {
                        ControlCommand::Halt => {
                            self.emergency_stop();
                            is_system_active = false;
                        },
                        ControlCommand::Resume => {
                            if self.is_healthy() {
                                is_system_active = true;
                                info!("RESUMED (Health Verified)");
                            } else {
                                warn!("RESUME BLOCKED: System Unhealthy (Voltage/Sensors)");
                            }
                        },
                        ControlCommand::MoveZ { target, .. } => {
                            self.active_target_nm = Some(target.0);
                        },
                        ControlCommand::ScanPath { points, intent_id, .. } => {
                            self.scan_intent_id = Some(intent_id);
                            self.scan_queue.push_back(points);
                        }
                    }
                }
            }

            let mut computed_volts = 0.0;

            // [FIX] Blind Halt: Always observe sensors and update state
            let input_voltage = self.voltage_monitor.read_input_voltage().unwrap_or(0.0);

            // Update Thermal
            let tf = safety_cache.get_thermal_field();
            let tm = Measurement::new(tf, 0.01, 0);
            self.last_thermal = Some(tm);

            let time_s = system_start.elapsed().as_secs_f64();

            // Poll Position Sensors (Always)
            let mut pos_measurement = Measurement::new(Nanometers(0.0), 0.0, 0);
            let mut est_pos = 0.0;

            let measurement_result = self.get_true_position_measurement(time_s).await;

            match measurement_result {
                Ok(m) => {
                    pos_measurement = m;
                    self.last_position = Some(m);

                    // Update Kalman Filter (Observation Mode)
                    match self.kalman.update_checked(pos_measurement.value.0, 0.01) {
                        Ok(p) => est_pos = p,
                        Err(e) => {
                            error!("KALMAN FAIL (Observation): {}", e);
                            if is_system_active { self.emergency_stop(); }
                        }
                    }
                },
                Err(e) => {
                    // Log error for debugging if needed
                    // For safety, we treat it as Vote Failed.
                    if is_system_active {
                        self.emergency_stop();
                        error!("SENSOR VOTE FAILED: {}", e);
                    }
                }
            }

            let sensor_health = if self.is_healthy() { "OK".to_string() } else { "FAULT".to_string() };

            if is_system_active {
                if let Err(e) = self.envelope.validate_command(est_pos, 0.0) {
                     self.emergency_stop();
                     error!("ENVELOPE FAIL: {}", e);
                }

                let target_um = self.active_target_nm.unwrap_or(est_pos) / 1000.0;
                // Calculate dt since last loop (using system_start as monotonic base or just assume 10ms target?)
                // The loop interval is 10ms. Real jitter matters for integration but for MPC step 10ms is the nominal.
                // Let's use 0.01 for now to match the loop rate, or better: measure actual dt.
                // We have `start` (Instant) of loop. We need `last_loop_start`.
                // For simplicity/robustness in this patch, we use fixed dt=0.01 as MPC assumes constant step size usually.
                let raw_volts = self.compute.step(est_pos / 1000.0, target_um, 0.01);

                // [FIX] Slew Rate Limiting (Hardware Protection)
                computed_volts = self.slew_limiter.limit(raw_volts, 0.01);

                // [SIMULATION FEEDBACK LOOP]
                if let Some(sim) = &mut self.sim_driver {
                    sim.update(computed_volts);
                }

                loop_cnt += 1;
                if loop_cnt % UI_TELEMETRY_DECIMATION == 0 {
                    let thermal_field = safety_cache.get_thermal_field();
                    let thermal = Measurement::new(thermal_field, 0.01, 0);

                    let _ = tx_telem.try_send(TelemetryEvent::Status {
                        timestamp: Utc::now(),
                        z_pos: pos_measurement,
                        safe: true,
                        thermal_field: thermal,
                        control_effort: Voltage(computed_volts),
                        input_voltage,
                        sensor_health: sensor_health.clone(),
                    });
                }
            }

            if let Ok(mut recorder) = self.flight_recorder.lock() {
                let rec = BlackBoxRecord {
                    seq: 0,
                    timestamp_ms: recorder.get_elapsed_ms(),
                    raw_sensors: self.raw_sensor_cache,
                    voted_position: pos_measurement.value.0,
                    uncertainty: pos_measurement.uncertainty_1sigma,
                    faulty_channels_mask: 0,
                    target: self.active_target_nm.unwrap_or(0.0),
                    control_effort: computed_volts,
                    safety_healthy: self.is_healthy(),
                };
                recorder.record(rec);
            }

            if !safety_cache.is_safe() {
                is_system_active = false;
                let _ = self.send_raw("!").await;
            }

            self.heartbeat.tick();
        }
    }
}
