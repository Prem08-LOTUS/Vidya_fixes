use std::sync::{
    atomic::{AtomicU64, AtomicBool, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{error, info, warn};

/// Thread-safe Heartbeat using Monotonic Clock
#[derive(Clone, Debug)]
pub struct Heartbeat {
    start: Instant,
    last_tick: Arc<AtomicU64>,
}

impl Heartbeat {
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            last_tick: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Update the heartbeat timestamp.
    /// MUST only be called after a successful control cycle.
    pub fn tick(&self) {
        let elapsed = self.start.elapsed().as_millis() as u64;
        self.last_tick.store(elapsed, Ordering::Relaxed);
    }

    /// Calculate time since last successful tick.
    pub fn age_ms(&self) -> u64 {
        let now = self.start.elapsed().as_millis() as u64;
        now.saturating_sub(self.last_tick.load(Ordering::Relaxed))
    }
}

/// Global "Emergency Stop" Latch
#[derive(Clone, Debug)]
pub struct SystemState {
    halted: Arc<AtomicBool>,
}

impl SystemState {
    pub fn new() -> Self {
        Self {
            halted: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn trigger_halt(&self) {
        self.halted.store(true, Ordering::SeqCst);
    }

    pub fn is_halted(&self) -> bool {
        self.halted.load(Ordering::SeqCst)
    }

    // Alias for compatibility if needed or clearer naming
    pub fn halt(&self) {
        self.trigger_halt();
    }
}

/// The Watchdog Task
pub async fn supervisor_task(
    heartbeat: Heartbeat,
    state: SystemState,
    max_age_ms: u64,
) {
    info!("SUPERVISOR: Watchdog Active (Timeout: {}ms)", max_age_ms);
    loop {
        sleep(Duration::from_millis(100)).await; // Poll 10Hz

        let age = heartbeat.age_ms();
        if age > max_age_ms {
            error!("CRITICAL: HEARTBEAT LOST ({}ms > {}ms) -> TRIGGERING EMERGENCY HALT", age, max_age_ms);
            state.trigger_halt();

            // Fix Defect #1: Orphaned Hardware State.
            // If the motion thread is dead (heartbeat lost), it cannot read the halt flag.
            // We must kill the process to force the OS/Hardware to de-energize (drop DTR/RTS).
            // This is the only safe way to handle a zombie controller thread.
            error!("SUPERVISOR: TERMINATING PROCESS TO ENSURE HARDWARE STOP");
            std::process::exit(1);
        }
    }
}
