#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use std::sync::{Arc, Mutex};
use tracing::{info, error, warn};
use tokio::sync::mpsc;
use std::time::Duration;
use std::path::PathBuf;

use agnix::motion_controller::{MotionController, PriorityCommand};
use agnix::persistence::{PersistenceManager, MotionIntent, RecoveryState, SystemSnapshot};
use agnix::types::{ControlCommand, TelemetryEvent, SystemStatus};
use agnix::physics::types::{Nanometers, ThermalField};
use agnix::sensor_monitor::SensorMonitor;
use agnix::supervisor::{Heartbeat, SystemState, supervisor_task};
use agnix::safety::state::SafetyState;
use agnix::gdsii_processor::GdsiiStreamingParser;

// [UPDATED] Use library config
use agnix::config::{self, SafetyConfig};

// Log Rotation Imports
use tracing_appender::rolling::{RollingFileAppender, Rotation};

// Global App State for Tauri
#[cfg(feature = "ui")]
struct AppState {
    persistence: Arc<PersistenceManager>,
    cmd_tx: mpsc::Sender<ControlCommand>,
    priority_tx: mpsc::UnboundedSender<PriorityCommand>,
    status: Arc<Mutex<SystemStatus>>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Log Rotation
    let file_appender = RollingFileAppender::new(
        Rotation::DAILY,
        "log",
        "agnix.log"
    );
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("AGNIX Control System Starting - Log Rotation Active");

    // [NEW] Load Config
    let safety_config = config::load_config("agnix.toml").unwrap_or_default();

    // 1. INIT PERSISTENCE
    let persistence = Arc::new(PersistenceManager::new("sqlite://./agnix.db").await?);

    // 2. CHECK RECOVERY
    let recovery = persistence.analyze_recovery().await?;
    let is_locked = !matches!(recovery, RecoveryState::CleanShutdown(_) | RecoveryState::FreshStart);

    // 3. CHANNELS
    let (cmd_tx, cmd_rx) = mpsc::channel::<ControlCommand>(16);
    let (priority_tx, priority_rx) = mpsc::unbounded_channel();
    let (telem_tx, mut telem_rx) = mpsc::channel::<TelemetryEvent>(1000);

    // 4. SHARED SAFETY STATE
    let safety_cache = Arc::new(SafetyState::new());

    // 5. THREAD D: METROLOGY (Background Poller)
    let monitor_ip = std::env::var("AGNIX_METROLOGY_IP").unwrap_or("127.0.0.1".to_string());
    let monitor = SensorMonitor::new(&monitor_ip);
    let safety_writer = safety_cache.clone();

    // Config for metrology limits
    let metrology_limit = safety_config.metrology_trust_limit_nm;

    tokio::spawn(async move {
        loop {
            let measurement = monitor.poll().await;
            let is_safe = measurement.trust_or_fail(metrology_limit).is_ok();
            let rh = 50.0;
            safety_writer.set_metrology_data(is_safe, measurement.value, rh);
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });

    // 6. THREAD C: LOGGER (Async Persistence & UI Updates)
    let p_logger = persistence.clone();
    let status_shared = Arc::new(Mutex::new(SystemStatus::default()));
    let status_logger = status_shared.clone();
    let safety_reader = safety_cache.clone();

    tokio::spawn(async move {
        let mut batch_timer = tokio::time::interval(Duration::from_millis(500));
        let mut last_state = SystemStatus::default();

        loop {
            tokio::select! {
                Some(event) = telem_rx.recv() => {
                    match event {
                        TelemetryEvent::Status { timestamp, z_pos, safe, thermal_field, control_effort: _, input_voltage, sensor_health } => {
                            if let Ok(mut s) = status_logger.lock() {
                                s.z_nm = z_pos.value.0;
                                s.z_sigma = z_pos.uncertainty_1sigma;
                                s.safe = safe;
                                s.timestamp = timestamp.to_rfc3339();
                                s.temperature = thermal_field.value.sensors[0];
                                s.thermal_gradient = thermal_field.value.gradient.norm();
                                s.thermal_field = Some(thermal_field.value);
                                s.input_voltage = input_voltage;
                                s.sensor_health = sensor_health;

                                let (_, rh) = safety_reader.get_metrology_data();
                                s.rh = rh;
                                last_state = s.clone();
                            }
                        }
                        TelemetryEvent::MotionComplete { intent_id } => {
                            let _ = p_logger.mark_intent_completed(&intent_id).await;
                        }
                        TelemetryEvent::MotionError { intent_id, error } => {
                            if let Ok(mut s) = status_logger.lock() {
                                s.last_error = Some(error);
                                s.safe = false;
                            }
                        }
                    }
                }
                _ = batch_timer.tick() => {
                    let snap = SystemSnapshot {
                        timestamp: chrono::DateTime::parse_from_rfc3339(&last_state.timestamp)
                            .unwrap_or(chrono::Utc::now().into())
                            .with_timezone(&chrono::Utc),
                        z_pos_nm: last_state.z_nm,
                        x_pos_um: 0.0,
                        y_pos_um: 0.0,
                        temperature_c: last_state.temperature,
                        job_id: None,
                        job_progress: 0.0,
                        safety_state: if last_state.safe { "SAFE".to_string() } else { "UNSAFE".to_string() },
                    };
                    let _ = p_logger.save_snapshot(snap).await;
                }
            }
        }
    });

    // 7. THREAD B: REAL-TIME MOTION
    info!("Spawning Motion Thread (Locked: {})", is_locked);

    let config_clone = safety_config.clone();

    let (heartbeat, sys_state) = spawn_motion_thread(
        cmd_rx,
        priority_rx,
        telem_tx.clone(),
        safety_cache.clone(),
        is_locked,
        config_clone
    ).await;

    // 7a. SPAWN THE SUPERVISOR
    tokio::spawn(supervisor_task(
        heartbeat,
        sys_state,
        500
    ));

    // 8. THREAD A: UI
    #[cfg(feature = "ui")]
    {
        let app_state = AppState {
            persistence,
            cmd_tx,
            priority_tx,
            status: status_shared,
        };
        start_ui(app_state);
    }

    #[cfg(not(feature = "ui"))]
    {
        tokio::signal::ctrl_c().await?;
    }

    Ok(())
}

async fn spawn_motion_thread(
    rx: mpsc::Receiver<ControlCommand>,
    priority_rx: mpsc::UnboundedReceiver<PriorityCommand>,
    tx: mpsc::Sender<TelemetryEvent>,
    safety: Arc<SafetyState>,
    start_locked: bool,
    config: SafetyConfig
) -> (Heartbeat, SystemState) {
    let mut motion = MotionController::new(0.0, config).expect("Init Failed");

    let hb = motion.heartbeat();
    let st = motion.system_state();

    let port_name = std::env::var("AGNIX_PORT").unwrap_or("/dev/ttyUSB0".to_string());
    if let Err(e) = motion.connect(&port_name) {
        warn!("Hardware connect failed (Sim Mode): {}", e);
    }

    tokio::spawn(async move {
        if let Err(e) = motion.run_rt_loop(rx, priority_rx, tx, safety, start_locked).await {
            error!("RT Loop Crashed: {}", e);
            std::process::exit(1);
        }
    });

    (hb, st)
}

// --- TAURI COMMANDS ---

#[cfg(feature = "ui")]
#[tauri::command]
async fn submit_move(
    state: tauri::State<'_, AppState>,
    target_nm: f64
) -> Result<(), String> {
    let intent = MotionIntent::MoveZ { target_nm, velocity_nm_s: 10.0 };
    let id = state.persistence.log_intent(&intent).await.map_err(|e| e.to_string())?;
    state.persistence.mark_intent_executing(&id).await.map_err(|e| e.to_string())?;

    state.cmd_tx.send(ControlCommand::MoveZ {
        target: Nanometers(target_nm),
        intent_id: id
    })
    .await
    .map_err(|_| "Failed to send command".to_string())?;

    Ok(())
}

#[cfg(feature = "ui")]
#[tauri::command]
async fn submit_job(
    state: tauri::State<'_, AppState>,
    gdsiiPath: String,
) -> Result<String, String> {
    let path = PathBuf::from(&gdsiiPath);
    let mut parser = GdsiiStreamingParser::new(&path).map_err(|e| e.to_string())?;
    let mut count = 0;

    let status_checker = state.status.clone();

    while let Some(poly) = parser.next_polygon().map_err(|e| e.to_string())? {
        if count % 100 == 0 {
            if let Ok(s) = status_checker.lock() {
                if !s.safe {
                    return Err("System Unsafe".to_string());
                }
            }
        }

        let intent = MotionIntent::ScanSegment {
            vertices: poly.clone(),
            velocity_um_s: 50.0,
        };

        let intent_id = state.persistence.log_intent(&intent).await.map_err(|e| e.to_string())?;

        // [FIX] Use send().await instead of try_send to apply backpressure instead of dropping
        if let Err(_) = state.cmd_tx.send(ControlCommand::ScanPath {
            points: poly,
            velocity_um_s: 50.0,
            intent_id,
        }).await {
             return Err("Command channel closed".to_string());
        }

        count += 1;
    }

    Ok(format!("{} scan paths loaded", count))
}

#[cfg(feature = "ui")]
#[tauri::command]
async fn perform_recovery(
    state: tauri::State<'_, AppState>,
    action: String
) -> Result<(), String> {
    info!("Recovery Action: {}", action);
    match action.as_str() {
        "RESUME" => state.cmd_tx.send(ControlCommand::Resume).await.map_err(|e| e.to_string())?,
        _ => state.priority_tx.send(PriorityCommand::EmergencyHalt).map_err(|e| e.to_string())?,
    }
    Ok(())
}

#[cfg(feature = "ui")]
#[tauri::command]
async fn get_recovery_state(state: tauri::State<'_, AppState>) -> Result<RecoveryState, String> {
    state.persistence.analyze_recovery().await.map_err(|e| e.to_string())
}

#[cfg(feature = "ui")]
#[tauri::command]
async fn get_status_update(
    state: tauri::State<'_, AppState>,
) -> Result<SystemStatus, String> {
    let status = state.status.lock().map_err(|_| "Failed to lock status".to_string())?;
    Ok(status.clone())
}

#[cfg(feature = "ui")]
#[tauri::command]
async fn get_system_health(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let status = state.status.lock().map_err(|_| "Failed to lock status".to_string())?;
    if status.safe {
        Ok("RUNNING".to_string())
    } else {
        Ok(if status.safe { "RUNNING".to_string() } else { "HALTED".to_string() })
    }
}

#[cfg(feature = "ui")]
fn start_ui(app_state: AppState) {
    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            submit_move,
            submit_job,
            perform_recovery,
            get_recovery_state,
            get_status_update,
            get_system_health
        ])
        .run(tauri::generate_context!())
        .expect("Tauri Error");
}
