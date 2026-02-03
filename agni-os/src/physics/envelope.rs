#[derive(Debug)]
pub struct PhysicsEnvelope {
    pub max_vel_nm_s: f64,
    pub max_acc_nm_s2: f64,
}

impl PhysicsEnvelope {
    pub fn check(
        &self,
        prev_pos: f64,
        curr_pos: f64,
        dt_s: f64,
    ) -> Result<(), &'static str> {
        if dt_s <= 0.0 {
            return Ok(()); // Ignore invalid dt
        }
        let vel = (curr_pos - prev_pos) / dt_s;
        if vel.abs() > self.max_vel_nm_s {
            return Err("Velocity limit exceeded");
        }
        Ok(())
    }
}
