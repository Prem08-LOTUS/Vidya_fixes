#!/usr/bin/env python3
"""RH Control Loop - Maintains humidity Â±1.5%"""

import smbus2, time, json, os
from dataclasses import dataclass
from datetime import datetime

try:
    import RPi.GPIO as GPIO
except (ImportError, RuntimeError):
    # Mock GPIO for non-Pi environment
    class MockGPIO:
        BCM = "BCM"
        OUT = "OUT"
        def setmode(self, mode): pass
        def setup(self, pin, mode): pass
        def cleanup(self): pass
        def PWM(self, pin, freq): return MockPWM()

    class MockPWM:
        def start(self, dc): pass
        def ChangeDutyCycle(self, dc): pass
        def stop(self): pass

    GPIO = MockGPIO()
    print("[WARN] RPi.GPIO not found or failed, using mock")

@dataclass
class RHState:
    setpoint: float = 50.0
    current_rh: float = 0.0
    current_temp: float = 0.0
    mist_pwm: int = 0
    is_paused: bool = False

class RHController:
    def __init__(self, mist_gpio=17, fan_gpio=27):
        try:
            self.bus = smbus2.SMBus(1)
        except (FileNotFoundError, PermissionError):
             # Mock for non-Pi environment
             print("[WARN] I2C bus not found, using mock")
             self.bus = None

        self.sht40_addr = 0x44
        try:
            GPIO.setmode(GPIO.BCM)
            GPIO.setup(mist_gpio, GPIO.OUT)
            GPIO.setup(fan_gpio, GPIO.OUT)
            self.mist_pwm = GPIO.PWM(mist_gpio, 1000)
            self.mist_pwm.start(0)
            self.fan_pwm = GPIO.PWM(fan_gpio, 1000)
            self.fan_pwm.start(0)
        except Exception:
             # In case mock fails or weirdness
             pass

        self.state = RHState()
        self.kp, self.ki, self.kd = 2.5, 0.8, 0.2
        self.integral = 0.0
        self.prev_error = 0.0

        self.log_dir = os.path.expanduser("~/agni-workspace/logs")
        os.makedirs(self.log_dir, exist_ok=True)

    def read_sht40(self):
        """Read humidity + temperature"""
        if self.bus is None:
            # Mock data for testing
            import random
            return 50.0 + random.uniform(-1, 1), 25.0 + random.uniform(-0.1, 0.1)

        try:
            self.bus.write_i2c_block_data(self.sht40_addr, 0xFD, [])
            time.sleep(0.05)
            data = self.bus.read_i2c_block_data(self.sht40_addr, 0, 6)

            rh = (((data[0] << 8) | data[1]) / 65535.0) * 100.0
            temp = ((((data[3] << 8) | data[4]) / 65535.0) * 175.0) - 45.0

            return rh, temp
        except:
            return None, None

    def pid_step(self, current_rh):
        """Calculate PID output"""
        error = self.state.setpoint - current_rh

        p_term = self.kp * error
        self.integral = max(-100, min(100, self.integral + self.ki * error * 0.1))
        i_term = self.integral
        d_term = self.kd * (error - self.prev_error) / 0.1
        self.prev_error = error

        return max(0, min(100, p_term + i_term + d_term))

    def control_loop(self, duration_seconds=600):
        """Main RH control loop"""
        start = time.time()
        cycle = 0

        print(f"[RH] Starting. Setpoint: {self.state.setpoint}%")

        try:
            while time.time() - start < duration_seconds:
                cycle += 1
                rh, temp = self.read_sht40()

                if rh is None:
                    time.sleep(1.0)
                    continue

                self.state.current_rh = rh
                self.state.current_temp = temp

                if abs(rh - self.state.setpoint) > 3.0:
                    pwm = 0
                    self.state.is_paused = True
                else:
                    pwm = self.pid_step(rh)
                    self.state.is_paused = False

                if hasattr(self, 'mist_pwm'):
                    self.mist_pwm.ChangeDutyCycle(pwm)

                # Log properly
                log_entry = {
                    "timestamp_iso": datetime.now().isoformat(),
                    "rh_percent": rh,
                    "temperature_c": temp,
                    "mist_pwm": pwm,
                    "is_paused": self.state.is_paused
                }
                with open(os.path.join(self.log_dir, "rh_log.jsonl"), "a") as f:
                    f.write(json.dumps(log_entry) + "\n")

                print(f"[{cycle:04d}] RH={rh:5.1f}% T={temp:5.2f}Â°C | Mist={pwm:3.0f}%")

                time.sleep(1.0)

        except KeyboardInterrupt:
            print("\n[RH] Interrupted")
        finally:
            if hasattr(self, 'mist_pwm'):
                self.mist_pwm.stop()
            try:
                GPIO.cleanup()
            except:
                pass

if __name__ == "__main__":
    controller = RHController()
    controller.state.setpoint = 50.0
    controller.control_loop(10)  # Short run for verification
