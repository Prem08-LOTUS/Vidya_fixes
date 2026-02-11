// agni-workspace/agni-os/src/telemetry/flight_recorder.rs
use serde::{Serialize, Deserialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::panic;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BlackBoxRecord {
    pub seq: u64,
    pub timestamp_ms: u64,

    // FORENSICS (E-7)
    pub raw_sensors: [f64; 3],    // What did the sensors say?
    pub voted_position: f64,      // What did we believe?
    pub uncertainty: f64,         // How sure were we?
    pub faulty_channels_mask: u8, // Bitmask of liars (1, 2, 4)

    // CONTROL
    pub target: f64,
    pub control_effort: f64,
    pub safety_healthy: bool,
}

// Fixed-size Ring Buffer (Allocation Free)
pub struct FlightRecorder<const N: usize> {
    buffer: [Option<BlackBoxRecord>; N],
    head: usize,
    seq: u64,
    start_time: std::time::Instant,
}

impl<const N: usize> FlightRecorder<N> {
    pub fn new() -> Self {
        Self {
            buffer: [None; N],
            head: 0,
            seq: 0,
            start_time: std::time::Instant::now(),
        }
    }

    pub fn record(&mut self, rec: BlackBoxRecord) {
        self.buffer[self.head] = Some(rec);
        self.head = (self.head + 1) % N;
        self.seq += 1;
    }

    pub fn dump_to_disk(&self, path: &str) -> std::io::Result<()> {
        let mut file = OpenOptions::new().write(true).create(true).truncate(true).open(path)?;
        // Dump oldest to newest
        for i in 0..N {
            let idx = (self.head + i) % N;
            if let Some(r) = self.buffer[idx] {
                serde_json::to_writer(&mut file, &r)?;
                file.write_all(b"\n")?;
            }
        }
        Ok(())
    }

    pub fn get_elapsed_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    pub fn install_panic_hook(recorder: Arc<Mutex<Self>>) {
        panic::set_hook(Box::new(move |info| {
            // Best effort lock
            if let Ok(mut rec) = recorder.lock() {
                let _payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
                    *s
                } else {
                    "Unknown Panic"
                };

                // Log a special "Panic" record
                let panic_record = BlackBoxRecord {
                    seq: u64::MAX,
                    timestamp_ms: rec.get_elapsed_ms(),
                    raw_sensors: [0.0; 3],
                    voted_position: 0.0,
                    uncertainty: f64::INFINITY,
                    faulty_channels_mask: 0xFF, // Full fault
                    target: 0.0,
                    control_effort: 0.0,
                    safety_healthy: false,
                };
                rec.record(panic_record);

                // Try to dump to disk immediately (Best Effort)
                let _ = rec.dump_to_disk("panic_dump.json");
            }
        }));
    }
}
