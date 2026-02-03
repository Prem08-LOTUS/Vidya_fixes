use crate::safety::uncertainty::Measurement;
use std::cmp::Ordering;

// A-7: Maximum allowed timestamp skew between redundant sensors
const MAX_TIMESTAMP_SKEW_CYCLES: u64 = 1;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SensorStatus {
    Valid,
    Degraded,
    Fault,
}

#[derive(Debug, Clone)]
pub struct VoteResult<T> {
    pub value: T,
    pub status: SensorStatus,
    pub faulty_channels: Vec<usize>,
}

pub fn voting_2oo3<T>(
    inputs: [Measurement<T>; 3],
    tolerance: f64,
) -> Option<VoteResult<T>>
where
    T: Copy + Into<f64> + From<f64> + PartialOrd,
{
    let vals: [f64; 3] = [
        inputs[0].value.into(),
        inputs[1].value.into(),
        inputs[2].value.into(),
    ];
    let times = [
        inputs[0].timestamp_cycle,
        inputs[1].timestamp_cycle,
        inputs[2].timestamp_cycle,
    ];

    // 1. Byzantine Defense: Finite Check
    if !vals.iter().all(|v| v.is_finite()) || !tolerance.is_finite() || tolerance <= 0.0 {
        return None;
    }

    // 2. Temporal Consistency
    let min_t = times.iter().min().unwrap_or(&0); // Safe: fixed-size array
    let max_t = times.iter().max().unwrap_or(&0);
    if (max_t - min_t) > MAX_TIMESTAMP_SKEW_CYCLES {
        return None;
    }

    // 3. SPAN-COLLAPSE ALGORITHM (SIL-4)
    // We do not trust pairwise checks. We calculate the spread of the agreeing set.
    // Sort values to easily find subgroups.
    let mut sorted = [(vals[0], 0), (vals[1], 1), (vals[2], 2)];
    sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));

    // Check strict 3-way agreement (Best Case)
    // Span = Max - Min. If Span <= Tolerance, all 3 agree.
    if (sorted[2].0 - sorted[0].0).abs() <= tolerance {
        return Some(VoteResult {
            value: T::from(sorted[1].0), // Median
            status: SensorStatus::Valid,
            faulty_channels: vec![],
        });
    }

    // Check 2-way agreement (Degraded Case)
    // We check the "inner" pairs of the sorted list.
    // If a pair agrees, the Median (sorted[1]) is always part of that pair (or between them).
    // Therefore, returning the Median is always correct and robust.

    // Pair 0-1 (Low and Median)
    if (sorted[1].0 - sorted[0].0).abs() <= tolerance {
        return Some(VoteResult {
            value: T::from(sorted[1].0), // Return Median
            status: SensorStatus::Degraded,
            faulty_channels: vec![sorted[2].1], // The High value is the outlier
        });
    }

    // Pair 1-2 (Median and High)
    if (sorted[2].0 - sorted[1].0).abs() <= tolerance {
        return Some(VoteResult {
            value: T::from(sorted[1].0), // Return Median
            status: SensorStatus::Degraded,
            faulty_channels: vec![sorted[0].1], // The Low value is the outlier
        });
    }

    // No consensus found (Span > Tolerance for all pairs)
    None
}

/// Rate-of-Change Limiter (SensorVoter)
pub struct SensorVoter {
    pub max_rate: f64,
    pub last_valid: Option<f64>,
    pub last_time: Option<f64>,
}

impl SensorVoter {
    pub fn new(max_rate: f64) -> Self {
        Self { max_rate, last_valid: None, last_time: None }
    }

    pub fn vote_checked<T>(&mut self, inputs: [Measurement<T>; 3], tol: f64, time: f64) -> Option<VoteResult<T>>
    where
        T: Copy + Into<f64> + From<f64> + PartialOrd
    {
        let result = voting_2oo3(inputs, tol)?;
        let val: f64 = result.value.into();

        if let (Some(last_v), Some(last_t)) = (self.last_valid, self.last_time) {
            let dt = time - last_t;
            if dt <= 0.0 { return None; }

            let rate = (val - last_v).abs() / dt;
            if rate > self.max_rate {
                return None;
            }
        }

        self.last_valid = Some(val);
        self.last_time = Some(time);
        Some(result)
    }
}
