# Proof of Correctness: 2oo3 Voting Algorithm

## 1. Algorithm Definition
The 2-out-of-3 (2oo3) voting logic is implemented in `agni-os/src/safety/voting.rs`.
It accepts 3 measurements `(v0, v1, v2)` and a `tolerance`.

## 2. Formal Logic
Let `|a - b| <= tolerance` be denoted as `Agrees(a, b)`.

### Case 1: Unanimous Agreement
*   `Agrees(v0, v1)` AND `Agrees(v1, v2)` AND `Agrees(v0, v2)`
*   **Result:** `Average(v0, v1, v2)`
*   **Status:** Valid

### Case 2: Single Fault (Masking)
*   **Scenario:** Sensor 2 fails (outlier).
*   `Agrees(v0, v1)` is TRUE.
*   `Agrees(v1, v2)` is FALSE.
*   `Agrees(v0, v2)` is FALSE.
*   **Result:** `Average(v0, v1)`
*   **Status:** Degraded (Faulty: {2})

### Case 3: Common Mode Failure (Divergence)
*   No pair agrees.
*   **Result:** `None`
*   **Action:** Immediate System HALT.

## 3. Implementation Constraints
To ensure mathematical validity, the generic type `T` must satisfy **Rule S-VOTE-01**:
> `T` must be a linear, lossless wrapper around `f64`.

This is enforced by the trait bounds `Into<f64>` and `From<f64>` on `Nanometers` and `Voltage`.
Non-linear types (e.g., Logarithmic Decibels) would violate the averaging logic `(a+b)/2` and are implicitly forbidden by this contract.

## 4. Verification
The `MotionController` integrates this logic in `get_voted_position`.
If `voting_2oo3` returns `None`, the controller executes `emergency_stop()` and sets `active = false`.
This proves **Fail-Closed** behavior for sensor divergence.
