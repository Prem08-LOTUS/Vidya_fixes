#[derive(Debug, Clone)]
pub struct SlewRateLimiter {
    max_slew_v_s: f64,
    last_voltage: f64,
}

impl SlewRateLimiter {
    pub fn new(max_slew_v_s: f64) -> Self {
        Self {
            max_slew_v_s,
            last_voltage: 0.0,
        }
    }

    pub fn limit(&mut self, target_voltage: f64, dt: f64) -> f64 {
        if dt <= 0.0 {
            return self.last_voltage;
        }

        let max_delta = self.max_slew_v_s * dt;
        let delta = target_voltage - self.last_voltage;

        let limited_delta = delta.clamp(-max_delta, max_delta);

        self.last_voltage += limited_delta;
        self.last_voltage
    }
}
