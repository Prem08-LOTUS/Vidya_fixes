// agni-workspace/agni-os/src/safety/voting.rs
use crate::safety::uncertainty::Measurement;
use tracing::warn;

// SIL-4 CONSTANTS
// Maximum allowed age of a sensor reading before we consider it "Dead"
const MAX_STALENESS_CYCLES: u64 = 5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SensorStatus {
    Valid,      // 3/3 Agreement
    Degraded,   // 2/3 Agreement (1 Fault)
    Fault,      // Disagreement or System Failure
}

#[derive(Debug, Clone, Copy)]
pub struct VoteResult {
    pub value: f64,
    pub status: SensorStatus,
    /// Bitmask: Bit 0 = Sensor A, Bit 1 = Sensor B, Bit 2 = Sensor C
    /// 1 means FAULTY. 0 means HEALTHY.
    pub faulty_channels_mask: u8,
}

/// The Iron Skeleton 2oo3 Voter (Median Filter).
///
/// PHYSICS:
/// 1. Sorts 3 inputs.
/// 2. Returns the Median if it agrees with *either* neighbor.
/// 3. Rejects 'Time Travel' (dt < 0) but accepts 'Hyper-Speed' (dt == 0).
pub fn voting_2oo3(
    inputs: [Measurement<f64>; 3],
    tolerance: f64,
) -> Option<VoteResult> {

    // 1. INPUT VALIDATION (Zero Trust)
    if tolerance <= 0.0 || !tolerance.is_finite() {
        return None; // Configuration Error
    }

    // Extract values and check for NaNs (Byzantine Fault)
    let vals = [inputs[0].value, inputs[1].value, inputs[2].value];
    if vals.iter().any(|v| !v.is_finite()) {
        return None; // Sensor Rail Failure (NaN)
    }

    // 2. SORT (The Median Strategy)
    // We create an index map to track which channel is which after sorting.
    // (Value, Original_Index)
    let mut sorted = [
        (vals[0], 0u8),
        (vals[1], 1u8),
        (vals[2], 2u8)
    ];

    // Unstable sort is fine and faster for 3 elements.
    // We use total_cmp because we already checked for NaNs.
    sorted.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));

    let low    = sorted[0];
    let median = sorted[1];
    let high   = sorted[2];

    // 3. EVALUATE CONSENSUS
    // Case A: Perfect 3-way agreement
    // If (High - Low) <= Tolerance, then everyone fits.
    if (high.0 - low.0) <= tolerance {
        return Some(VoteResult {
            value: median.0,
            status: SensorStatus::Valid,
            faulty_channels_mask: 0,
        });
    }

    // Case B: 2-way agreement (Median + Low)
    // If the median agrees with the low side, but high is an outlier.
    if (median.0 - low.0) <= tolerance {
        return Some(VoteResult {
            value: median.0,
            status: SensorStatus::Degraded,
            faulty_channels_mask: 1 << high.1, // The High sensor is the liar
        });
    }

    // Case C: 2-way agreement (Median + High)
    // If the median agrees with the high side, but low is an outlier.
    if (high.0 - median.0) <= tolerance {
        return Some(VoteResult {
            value: median.0,
            status: SensorStatus::Degraded,
            faulty_channels_mask: 1 << low.1, // The Low sensor is the liar
        });
    }

    // Case D: Total Chaos (No 2 sensors agree)
    None
}

/// Rate-of-Change Limiter & Staleness Guard
pub struct SensorVoter {
    pub max_rate: f64,
    pub last_valid: Option<f64>,
    pub last_time: Option<f64>,
}

impl SensorVoter {
    pub fn new(max_rate: f64) -> Self {
        Self { max_rate, last_valid: None, last_time: None }
    }

    pub fn vote_checked(
        &mut self,
        inputs: [Measurement<f64>; 3],
        tol: f64,
        time: f64
    ) -> Option<VoteResult> {

        // 1. Run the Static Voter
        let result = voting_2oo3(inputs, tol)?;

        // 2. Temporal Checks
        if let (Some(last_v), Some(last_t)) = (self.last_valid, self.last_time) {
            let dt = time - last_t;

            // [FIX] The Time Stopper Bug
            // OLD: if dt <= 0.0 { return None; }
            // NEW: We only reject negative time (Time Travel).
            if dt < 0.0 {
                warn!("VOTER: Time Travel Detected (dt={:.6})", dt);
                return None;
            }

            // If dt == 0 (Hyper-speed), we skip velocity check but ACCEPT the value.
            if dt > 0.0 {
                let velocity = (result.value - last_v).abs() / dt;

                // Physics Check: Teleportation
                if velocity > self.max_rate {
                    warn!("VOTER: Physics Violation (v={:.2} > max={:.2})", velocity, self.max_rate);
                    return None;
                }
            }
        }

        // 3. Staleness Check (Liveness)
        // If all sensors have old timestamps, the hardware is frozen.
        let min_cycle = inputs.iter().map(|m| m.timestamp_cycle).min().unwrap_or(0);
        let max_cycle = inputs.iter().map(|m| m.timestamp_cycle).max().unwrap_or(0);

        if (max_cycle - min_cycle) > MAX_STALENESS_CYCLES {
             warn!("VOTER: Sensor Desync Detected");
             // We continue, trusting the median, but log the warning.
        }

        self.last_valid = Some(result.value);
        self.last_time = Some(time);

        Some(result)
    }
}
