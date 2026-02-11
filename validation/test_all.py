#!/usr/bin/env python3
"""Comprehensive automated test suite"""

import sys
import os
# Ensure imports work relative to workspace root
sys.path.insert(0, os.path.abspath('agni-workspace'))

from simulation.climate_controller_hardened_v2 import (
    HardenedClimateController, ControllerConfig, EnvironmentMeasurement,
    EnvironmentState, ActuatorCommand, monotonic_ms
)
from src.crypto import HmacAuthenticator
from src.voting import ByzantineVoter, SensorReading, VotingResult
from datetime import datetime
import time

def test_humidity_setpoint():
    """Test that humidity_setpoint property exists and validates"""
    cfg = ControllerConfig()
    c = HardenedClimateController(cfg)

    # Valid
    c.humidity_setpoint = 50.0
    assert c.humidity_setpoint == 50.0

    # Invalid: out of range
    try:
        c.humidity_setpoint = 200.0
        assert False, "Should reject 200% humidity"
    except ValueError:
        pass

    # Invalid: NaN
    try:
        c.humidity_setpoint = float('nan')
        assert False, "Should reject NaN"
    except ValueError:
        pass

    print("âœ“ test_humidity_setpoint PASS")

def test_monotonic_freshness():
    """Test freshness using monotonic clock"""
    cfg = ControllerConfig(loop_hz=10.0, max_data_age_ms=500)
    c = HardenedClimateController(cfg)

    # Fresh measurement
    now = monotonic_ms()
    m = EnvironmentMeasurement(25.0, 50.0, datetime.utcnow(), now)
    assert m.is_fresh(now, 500), "Fresh measurement should pass"

    # Stale measurement
    old = now - 600
    m_stale = EnvironmentMeasurement(25.0, 50.0, datetime.utcnow(), old)
    assert not m_stale.is_fresh(now, 500), "Stale measurement should fail"

    print("âœ“ test_monotonic_freshness PASS")

def test_fail_safe_command():
    """Test that fail-safe actuation works"""
    cmd_safe = ActuatorCommand.safe()
    assert cmd_safe.heater_pwm == 0.0
    assert cmd_safe.mist_pwm == 0.0
    assert cmd_safe.fan_pwm == 0.0

    print("âœ“ test_fail_safe_command PASS")

def test_lockdown_latch():
    """Test that lockdown latches until reset"""
    cfg = ControllerConfig()
    c = HardenedClimateController(cfg)

    # Inject stale data
    old_time = monotonic_ms() - 1000  # 1 second old
    c.measurement = EnvironmentMeasurement(25.0, 50.0, datetime.utcnow(), old_time)

    # Step should trigger lockdown
    meas, cmd, st = c.step()
    assert st == EnvironmentState.LOCKDOWN
    assert cmd == ActuatorCommand.safe()

    # Latch should be set
    assert c.lockdown_latch == True

    # Reset
    c.reset_lockdown()
    assert c.lockdown_latch == False

    print("âœ“ test_lockdown_latch PASS")

def test_hmac():
    """Test HMAC signing and verification"""
    secret = b"test-secret-key-minimum-32-bytes-long"
    auth = HmacAuthenticator(secret)

    payload = {"temp": 25.0, "rh": 50.0}
    signed = auth.sign(payload)

    # Verify should succeed
    verified = auth.verify(signed)
    assert verified == payload

    # Tampered should fail
    signed["payload"]["temp"] = 100.0
    tampered = auth.verify(signed)
    assert tampered is None

    print("âœ“ test_hmac PASS")

def test_byzantine_voting():
    """Test 3-of-3 consensus voting"""
    voter = ByzantineVoter()

    # Consensus
    voter.add_reading(SensorReading(1, 25.0, 50.0, True))
    voter.add_reading(SensorReading(2, 25.1, 50.1, True))
    voter.add_reading(SensorReading(3, 24.9, 49.9, True))

    result, value = voter.vote()
    assert result == VotingResult.CONSENSUS
    assert value[0] == 25.0  # Median

    # No consensus (faulty sensor)
    voter.readings = []
    voter.add_reading(SensorReading(1, 25.0, 50.0, True))
    voter.add_reading(SensorReading(2, 100.0, 95.0, True))
    voter.add_reading(SensorReading(3, 25.1, 50.1, True))

    result, value = voter.vote()
    assert result == VotingResult.NO_CONSENSUS

    print("âœ“ test_byzantine_voting PASS")

if __name__ == "__main__":
    tests = [
        test_humidity_setpoint,
        test_monotonic_freshness,
        test_fail_safe_command,
        test_lockdown_latch,
        test_hmac,
        test_byzantine_voting,
    ]

    failed = 0
    for test in tests:
        try:
            test()
        except Exception as e:
            print(f"âœ— {test.__name__} FAIL: {e}")
            failed += 1

    print(f"\n{len(tests)-failed}/{len(tests)} tests passed")
    sys.exit(0 if failed == 0 else 1)
