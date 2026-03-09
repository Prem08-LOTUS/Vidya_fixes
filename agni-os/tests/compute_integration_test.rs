//! Integration test: Sensor Monitor + Kalman + MPC + Motion

#[cfg(test)]
mod compute_integration {
    use agnix::compute::ComputeEngine; // Renamed from agni_os to agnix
    use rand;

    #[test]
    fn test_e2e_z_tracking() {
        // Simulate: approach from Z=0 to target Z=50Âµm
        let mut engine = ComputeEngine::new(0.0, 0.01);
        let target = 50.0;

        let mut positions = Vec::new();

        for step in 0..200 {
            // Simulate sensor reading (true position + noise)
            let true_z = 50.0 * (step as f64 / 200.0);  // Ramp 0->50Âµm (simulating physics moving it?)

            let noisy_z = true_z + (rand::random::<f64>() - 0.5) * 0.5;

            // Compute step
            let _voltage = engine.step(noisy_z, target);

            let est_z = engine.position();
            positions.push(est_z);
        }

        // Check convergence
        let final_z = positions[positions.len() - 1];
        // At step 200, true_z is 50.0.
        // Kalman should estimate ~50.0.
        assert!((final_z - target).abs() < 2.0,
                "Failed to converge: final={}, target={}", final_z, target);
    }

    #[test]
    fn test_stability_against_noise() {
        let mut engine = ComputeEngine::new(25.0, 0.01);
        let target = 25.0;  // Stay still

        // 50 steps of high noise
        for _ in 0..50 {
            let noisy = 25.0 + (rand::random::<f64>() - 0.5) * 2.0;  // Â±1Âµm noise
            engine.step(noisy, target);
        }

        // Should not drift
        assert!((engine.position() - target).abs() < 1.0);

        assert!(engine.velocity().abs() < 1.0, "Velocity drift too high: {}", engine.velocity());
    }
}
