use std::sync::Mutex;
use std::time::Instant;

pub struct SimulatedMotionDriver {
    pub position_um: f64,
    pub velocity_um_s: f64,
    pub last_update: Instant,
}

impl SimulatedMotionDriver {
    pub fn new() -> Self {
        Self {
            position_um: 0.0,
            velocity_um_s: 0.0,
            last_update: Instant::now(),
        }
    }

    pub fn update(&mut self, voltage: f64) -> f64 {
        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f64();
        self.last_update = now;

        if dt <= 0.0 { return self.position_um; }

        // Simple Physics Model: 1V = 1000 um/s^2 acceleration (roughly)
        // Damping: Air resistance
        let coupling = 1000.0;
        let damping = 0.1;

        let acc = (voltage * coupling) - (self.velocity_um_s * damping);

        self.velocity_um_s += acc * dt;
        self.position_um += self.velocity_um_s * dt;

        // Hard Stops
        if self.position_um < -10.0 {
            self.position_um = -10.0;
            self.velocity_um_s = 0.0;
        }
        if self.position_um > 1000.0 {
            self.position_um = 1000.0;
            self.velocity_um_s = 0.0;
        }

        self.position_um
    }
}
