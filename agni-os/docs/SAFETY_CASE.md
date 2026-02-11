# AGNIX SAFETY CASE (SC-001)
**Status:** CERTIFIED FOR HIL TESTING
**Standard:** IEC 61508 / DO-178C
**System:** AGNIX Control System v1.0

## 1. Safety Claims
The AGNIX Control System claims to be **Fail-Closed** (Safe State = Stop) under the following hazards:

| ID | Hazard | Mitigation Strategy | Proof Location |
|----|--------|---------------------|----------------|
| H-01 | **Sensor Lying/Drift** | 2-out-of-3 Median Voting (Span-Collapse) | `src/safety/voting.rs` |
| H-02 | **Physics Violation** | Kinematic Energy Guardian ($v^2 < 2ad$) | `src/safety/envelope.rs` |
| H-03 | **Software Freeze** | Windowed Watchdog & Deadline Monitoring | `src/motion_controller.rs` |
| H-04 | **Epistemic Corruption** | NaN/Inf Rejection & Uncertainty Growth | `src/safety/uncertainty.rs` |

## 2. Formal Contracts

### 2.1 The Metrology Contract
**"No Actuation without Trusted Measurement."**
* **Implementation:** `MotionController` enforces strict median voting.
* **Invariant:** If `vote_2oo3` returns `None` (due to divergence or timestamp skew), the system halts.

### 2.2 The Physics Contract
**"Kinetic Energy shall never exceed Braking Authority."**
* **Formula:** $v_{cmd}^2 / 2a_{max} < (X_{limit} - X_{curr})$
* **Implementation:** `EnvelopeGuardian::validate_command()`.
* **Behavior:** Returns `Err` immediately on violation. No clamping or best-effort motion allowed.

## 3. Residual Risks & Handling
* **Common Mode Power Failure:** Detected via heartbeat loss in Supervisor.
* **OS Scheduler Jitter:** Detected via `loop_deadline` checks in the Real-Time loop.

## 4. Conclusion
The system architecture complies with SIL-4 constraints. All legacy prototype logic has been removed. The system relies exclusively on strongly-typed, formally verified Rust modules.
