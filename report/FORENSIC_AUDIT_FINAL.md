# 🔥 AGNIX — FORENSIC AUDIT REPORT

**Date:** 2024-05-22
**Auditor:** Jules (Lead Adversarial Systems Auditor)
**Verdict:** ❌ UNSAFE — DO NOT POWER HARDWARE

---

## 1. Executive Summary

The AGNIX system, while superficially robust with SIL-4 architectural patterns (2oo3 voting, CBFs, Envelope Guardians), is **functionally hollow**. The critical safety logic verified in unit tests is **bypassed** in the runtime application. The Motion Controller ignores sensor voting and relies on hardcoded simulated values (`0.0`). The Sensor Monitor performs a handshake check that updates no state. The Configuration loader ignores the config file. The Voltage Monitor is a mock. This system is a high-fidelity **simulator** masquerading as a control system. Powering real hardware with this code would result in immediate, uncontrolled motion as the controller believes it is always at position `0.0` regardless of reality.

---

## 2. File-by-File Findings

*   **`agni-os/src/motion_controller.rs`**: **CRITICAL.** Defines `voter` but never uses it. Defines `voltage_monitor` but uses a mocked implementation. `get_true_position_measurement` returns hardcoded `0.0`.
*   **`agni-os/src/sensor_monitor.rs`**: **DEFECT.** `handshake_complete` field is dead code. `perform_handshake` is called on every poll (inefficient) but never updates the state.
*   **`agni-os/src/config.rs`**: **DEFECT.** `load_config` reads the file but discards the content, returning `Default::default()`.
*   **`agni-os/src/io/voltage_monitor.rs`**: **MOCK.** Returns `Ok(24.0)` unconditionally. Brownout protection is effectively disabled in reality.
*   **`agni-os/src/supervisor.rs`**: **PASS.** Logic is sound. Process termination on heartbeat loss is the only verified safety mechanism.
*   **`ui/src/App.tsx`**: **PASS.** UI logic respects the `safe` flag, but the flag is derived from flawed backend data.
*   **`ui/src/components/RecoveryScreen.tsx`**: **PASS.** correctly disables "RESUME" on critical failures.

---

## 3. Critical Defects

1.  **Phantom Voting (Severity 1):** The `MotionController` initializes a `SensorVoter` but **never calls it** in `run_rt_loop`. The system relies on `get_true_position_measurement` which returns a hardcoded `0.0`.
    *   *Consequence:* The PID/MPC loop will apply infinite voltage trying to correct position if the real position is not 0, or zero voltage if it is, with no feedback.
2.  **Blind Voltage Monitor (Severity 1):** `VoltageMonitor` is a hardcoded mock returning `24.0V`.
    *   *Consequence:* Brownouts will be ignored. Hardware will undervoltage without a safe stop.
3.  **Config Placebo (Severity 2):** `config.rs` ignores `agnix.toml`.
    *   *Consequence:* Tuning safety parameters (velocity limits) has no effect. The system runs on compiled-in defaults.

---

## 4. Non-Critical Risks

1.  **Channel Starvation:** `cmd_tx` has a capacity of 16. `submit_job` floods this channel. While `try_send` prevents memory leaks, it causes valid jobs to fail if the parser outruns the motion loop (likely).
2.  **Stateless Handshake:** `SensorMonitor` re-verifies the hardware handshake on every 10ms poll cycle.

---

## 5. Dead Code / Illusions

*   **`MotionController.voter`**: Initialized, never read. The entire 2oo3 voting subsystem is dead code in the runtime.
*   **`SensorMonitor.handshake_complete`**: Never written to `true`.
*   **`config::load_config`**: File reading logic is dead; result is discarded.

---

## 6. UI/UX Failures

*   **False Confidence:** The UI shows "SAFE" and "RUNNING" based on mocked data.
*   **Double Recovery Modal:** The recovery screen renders twice in `App.tsx` (once in the Halted guard, once in the main layout).

---

## 7. Safety Violations

*   **Violation of "Safety Dominance":** The 2oo3 Voting logic (the primary sensor safety mechanism) is disconnected. The system operates on "Single Point of Failure" (the hardcoded mock).
*   **Violation of "Real-Time I/O":** Input is mocked.

---

## 8. Proof of Correctness

*   **Formal Methods:** The `safety_properties.rs` tests passed, proving the *Voting Algorithm* is correct.
*   **Integration:** The *usage* of that algorithm is **proven absent** by the compiler warning: `field voter is never read`.

---

## 9. Final Verdict

# ❌ UNSAFE — DO NOT POWER HARDWARE

**Justification:** The system is partially mocked and key safety components (Voting, Voltage Monitoring) are not wired into the control loop. It is a simulation, not a control system.
