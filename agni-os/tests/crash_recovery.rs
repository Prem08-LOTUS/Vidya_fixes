use agnix::persistence::{PersistenceManager, MotionIntent, SystemSnapshot, RecoveryState};
use chrono::Utc;
use uuid::Uuid;

#[tokio::test]
async fn test_power_loss_during_motion() {
    // 1. Setup DB (In-Memory for test speed)
    let pm = PersistenceManager::new("sqlite::memory:").await.unwrap();

    // 2. Simulate System Running (Snapshot 1)
    let snap = SystemSnapshot {
        timestamp: Utc::now(),
        z_pos_nm: 100.0,
        x_pos_um: 10.0,
        y_pos_um: 10.0,
        temperature_c: 25.0,
        job_id: Some(Uuid::new_v4()),
        job_progress: 0.5,
        safety_state: "ACTIVE".into(),
    };
    pm.save_snapshot(snap.clone()).await.unwrap();

    // 3. Log Intent (Move Z to 200nm)
    let intent = MotionIntent::MoveZ { target_nm: 200.0, velocity_nm_s: 10.0 };
    let id = pm.log_intent(&intent).await.unwrap();

    // 4. Mark Executing
    pm.mark_intent_executing(&id).await.unwrap();

    // 5. SIMULATE POWER CUT (We never call mark_completed)
    // ... System "reboots" by analyzing the DB state ...

    // 6. Analyze Recovery
    let recovery = pm.analyze_recovery().await.unwrap();

    // 7. Assert "Power Loss Detected"
    match recovery {
        RecoveryState::PowerLossDuringMotion { last_safe_snapshot, interrupted_intent } => {
            assert_eq!(last_safe_snapshot.z_pos_nm, 100.0);
            match interrupted_intent {
                MotionIntent::MoveZ { target_nm, .. } => assert_eq!(target_nm, 200.0),
                _ => panic!("Wrong intent type"),
            }
            println!("TEST PASSED: Detected Interrupted Motion Z=100nm -> 200nm");
        },
        _ => panic!("System failed to detect power loss during motion! Got: {:?}", recovery),
    }
}
