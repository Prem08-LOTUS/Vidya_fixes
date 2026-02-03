# AGNIX AI AUDIT REPORT
**Auditor:** Lead Safety-Critical Systems Auditor (SIL-4 Certified)
**System:** AGNIX Control System (Motion & Metrology)
**Status:** **CRITICAL FAIL**

---

## Critical Findings

### 1. Non-Canonical Security Serialization (HMAC Bypass)
- **Location:** `agni-os/src/sensor_monitor.rs`: Line 85–100.
- **Description:** The system reconstructs a JSON string from a `BTreeMap` to verify the HMAC signature. In SIL-4 systems, JSON is **never** an acceptable format for cryptographic signing because floating-point serialization is non-deterministic (e.g., `25.0` vs `25` vs `2.5e1`). A mismatch in the `serde_json` serializer version between the Python Metrology Server and the Rust client will result in a "Safety DoS," where the system halts because it cannot verify valid signatures.
- **Fix:** Use a deterministic binary serialization format like **CBOR** or **Postcard** for the signed payload, or sign the raw bytes received before any UTF-8/JSON parsing.

### 2. Orphaned Actuators on Python Control Failure
- **Location:** `metrology/rh_controller.py`: Line 118–130; `metrology/server.py`.
- **Description:** The Python metrology loops run in `threading.Thread` with a 31,536,000-second duration. Python’s Global Interpreter Lock (GIL) makes these loops non-deterministic. If the FastAPI `uvicorn` worker saturates the CPU or if the `rh_ctrl` thread encounters a `RuntimeError` (e.g., I2C bus hang), the `mist_pwm` may be left in its last "ON" state. This creates a "Humidity Runaway" scenario, potentially shorting the piezo actuators via condensation.
- **Fix:** Implement a hardware-level Watchdog Timer (WDT) on the Raspberry Pi GPIO that requires a "pet" signal from the Python thread every 100ms. If the thread hangs, the WDT must pull the GPIO `LOW` via a physical pull-down resistor.

### 3. Production Safety Bypass via Environment Variable
- **Location:** `agni-os/src/io/voltage_monitor.rs`: Line 14–20.
- **Description:** The system allows disabling brownout protection via `AGNIX_UNSAFE_IGNORE_VOLTAGE`. While intended for "dev," this logic exists in the production-compiled binary. This is a severe **Safety Bypass (E-2)**. An attacker or an operator error setting this variable allows the system to operate during a voltage sag, leading to unpredictable piezo behavior and potential mechanical collision.
- **Fix:** Use `#[cfg(debug_assertions)]` to wrap the environment variable check, ensuring it is physically impossible to bypass voltage monitoring in a release build.

### 4. Timestamp Truncation / Wraparound Vulnerability
- **Location:** `agni-os/src/sensor_monitor.rs`: Line 117; `agni-os/src/supervisor.rs`: Line 28.
- **Description:** `SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64`. The `as_millis()` method returns a `u128`. Casting to `u64` truncates the value. While the 64-bit millisecond counter won't wrap for 500 million years, the inconsistency in types across the metrology bridge (Python uses floats, Rust uses truncated integers) can lead to precision loss in the `abs_diff` freshness check, allowing replay attacks of old sensor data within a specific jitter window.
- **Fix:** Use `u128` consistently for millisecond timestamps or use `Instant` for internal monotonic tracking to avoid "Time-of-Check to Time-of-Use" (TOCTOU) logic errors.

### 5. Silent Logic Failure in GDSII Record Parsing
- **Location:** `agni-os/src/gdsii_processor.rs`: Line 54–55.
- **Description:** The parser uses `self.cursor.read_u8().unwrap()`. Although a manual bounds check is performed previously, the use of `.unwrap()` in a safety-critical parser is a violation of the crate's `#![deny(clippy::unwrap_used)]`. If a malformed GDSII file bypasses the length check (e.g., via an integer underflow in `record_len - 4`), the system will panic. A panic in the `gdsii_processor` can leave the `MotionController` in an indeterminate state if the heartbeat is still ticking.
- **Fix:** Replace `.unwrap()` with `?` or `.map_err()`. All I/O must be treated as untrusted.

### 6. Integer Overflow in "Unbounded" Buffer Mitigation
- **Location:** `agni-os/src/io/grbl_reader.rs`: Line 21.
- **Description:** `if self.buf.len() + data.len() > MAX_BUFFER_SIZE`. This is susceptible to integer overflow. If `data.len()` is extremely large (malicious serial input), the sum can wrap around, bypassing the size check and leading to an Out-of-Memory (OOM) abort via the `buf.push_str` call.
- **Fix:** Use `checked_add` or compare against the remaining capacity: `if data.len() > MAX_BUFFER_SIZE - self.buf.len()`.

### 7. Floating Point Non-Determinism in MPC Solver
- **Location:** `agni-os/src/compute/mpc.rs`: Line 36–66.
- **Description:** The MPC solver uses a 20-iteration gradient descent. In safety-critical loops, the convergence of floating-point math is non-deterministic. If the "coupling" constant or the error result produces a `NaN` (due to subnormal float issues), the `u_candidate -= learning_rate * gradient` will propagate that `NaN` to the hardware voltage output.
- **Fix:** Add a `finite()` check inside the iteration loop. If any intermediate value becomes `non-finite`, the solver must return 0.0V and trigger a `Halt`.

### 8. Liveness Hazard: Blocking I/O in Persistence
- **Location:** `agni-os/src/persistence/mod.rs`: Line 106.
- **Description:** `save_snapshot` performs an `Arc<Mutex<Option<SystemSnapshot>>>` lock, followed by an `INSERT` and a `DELETE` on a SQLite database. This is called from the "Logger" thread. However, the UI thread also locks this mutex via `get_status_update`. If SQLite performs a "Checkpoint" (WAL mode), the Logger thread will block. This causes the UI to hang, preventing the operator from seeing that the system is unsafe or clicking the "Emergency Halt" button.
- **Fix:** Use a `tokio::sync::RwLock` or separate the "Latest Snapshot" (in-memory) from the "Persistence DB" (disk-write) to ensure UI telemetry is never blocked by disk I/O.

---

## Audit Summary
The AGNIX system demonstrates high-quality Rust idioms but fails SIL-4 certification due to **non-deterministic timing in Python** and **cryptographic fragility** in the Metrology link. The system's safety-critical path (Motion) is well-guarded by the `EnvelopeGuardian`, but the "Metrology Server" is a weak point that can be exploited to feed the controller "Safe" signed data while the physical environment is failing.

**Recommendation:** **DO NOT DEPLOY** until the Python Metrology Server is replaced with a real-time RTOS implementation (e.g., Rust on STM32) and canonical serialization is enforced.