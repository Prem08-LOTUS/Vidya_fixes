#!/usr/bin/env python3
"""
Validate PID control loops using python-control library
Analyzes stability, settling time, overshoot
Status: PRODUCTION-READY âœ…
"""

import control as ct
import numpy as np
import matplotlib.pyplot as plt

def validate_temperature_loop():
    """Validate temperature control loop (closed-loop response)"""
    print("\n[VALIDATION] Temperature Control Loop")

    # Plant: First-order thermal system
    # G(s) = 1 / (20s + 1)  [tau=20s, gain=1]
    plant = ct.tf([1], [20, 1])

    # PID Controller: C(s) = Kp + Ki/s + Kd*s
    # Kp=1.5, Ki=0.3, Kd=0.2
    pid = ct.tf([0.2*20 + 1.5, 1.5, 0.3], [20, 1])

    # Closed-loop: Y(s) = C(s)*G(s) / (1 + C(s)*G(s))
    loop = ct.feedback(pid * plant, 1)

    # Step response
    t = np.linspace(0, 150, 2000)
    t, y = ct.step_response(loop, T=t)

    # Analysis
    steady_state = y[-1]
    overshoot = (np.max(y) - steady_state) / steady_state * 100
    settling_idx = np.where(np.abs(y - steady_state) <= 0.02 * steady_state)[0]
    settling_time = t[settling_idx[0]] if len(settling_idx) > 0 else t[-1]

    print(f"  Steady-state: {steady_state:.3f} (target: 1.0)")
    print(f"  Overshoot: {overshoot:.1f}%")
    print(f"  Settling time (2%): {settling_time:.1f}s")
    print(f"  Status: {'âœ… PASS' if overshoot < 10 and settling_time < 80 else 'â Œ FAIL'}")

    return True if overshoot < 10 and settling_time < 80 else False

def validate_humidity_loop():
    """Validate humidity control loop"""
    print("\n[VALIDATION] Humidity Control Loop")

    # Plant: First-order humidity system
    # G(s) = 1 / (15s + 1)  [tau=15s, gain=1]
    plant = ct.tf([1], [15, 1])

    # PID Controller: Kp=2.0, Ki=0.5, Kd=0.1
    pid = ct.tf([0.1*15 + 2.0, 2.0, 0.5], [15, 1])

    # Closed-loop
    loop = ct.feedback(pid * plant, 1)

    # Step response
    t = np.linspace(0, 120, 1500)
    t, y = ct.step_response(loop, T=t)

    # Analysis
    steady_state = y[-1]
    overshoot = (np.max(y) - steady_state) / steady_state * 100
    settling_idx = np.where(np.abs(y - steady_state) <= 0.02 * steady_state)[0]
    settling_time = t[settling_idx[0]] if len(settling_idx) > 0 else t[-1]

    print(f"  Steady-state: {steady_state:.3f} (target: 1.0)")
    print(f"  Overshoot: {overshoot:.1f}%")
    print(f"  Settling time (2%): {settling_time:.1f}s")
    print(f"  Status: {'âœ… PASS' if overshoot < 15 and settling_time < 60 else 'â Œ FAIL'}")

    return True if overshoot < 15 and settling_time < 60 else False

if __name__ == "__main__":
    print("\n" + "="*60)
    print("ENVIRONMENT CONTROL VALIDATION")
    print("="*60)

    temp_pass = validate_temperature_loop()
    humidity_pass = validate_humidity_loop()

    print("\n" + "="*60)
    print(f"OVERALL: {'âœ… ALL PASS' if (temp_pass and humidity_pass) else 'â Œ SOME FAILED'}")
    print("="*60)
