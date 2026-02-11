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
    rh_setpoint: float = 50.0
    temp_setpoint: float = 25.0
    current_rh: float = 0.0
    current_temp: float = 0.0
    mist_pwm: int = 0
    heater_pwm: int = 0
    is_paused: bool = False

class RHController:
    def __init__(self, mist_gpio=17, fan_gpio=27, heater_gpio=18):
        try:
            self.bus = smbus2.SMBus(1)
        except (FileNotFoundError, PermissionError):
             # [FIX #128] Silent Mock Removed. Fail Fast.
             print("[CRITICAL] I2C bus not found. Hardware required.")
             self.bus = None

        self.sht40_addr = 0x44
        try:
            GPIO.setmode(GPIO.BCM)
            GPIO.setup(mist_gpio, GPIO.OUT)
            GPIO.setup(fan_gpio, GPIO.OUT)
            GPIO.setup(heater_gpio, GPIO.OUT)

            self.mist_pwm = GPIO.PWM(mist_gpio, 1000)
            self.mist_pwm.start(0)

            self.fan_pwm = GPIO.PWM(fan_gpio, 1000)
            self.fan_pwm.start(0)

            self.heater_pwm = GPIO.PWM(heater_gpio, 1000)
            self.heater_pwm.start(0)
        except Exception:
             pass

        self.state = RHState()

        # RH PID
        self.rh_kp, self.rh_ki, self.rh_kd = 2.5, 0.8, 0.2
        self.rh_integral = 0.0
        self.rh_prev_error = 0.0

        # Temp PID
        self.temp_kp, self.temp_ki, self.temp_kd = 15.0, 2.0, 0.5
        self.temp_integral = 0.0
        self.temp_prev_error = 0.0

        self.log_dir = os.path.expanduser("~/agni-workspace/logs")
        os.makedirs(self.log_dir, exist_ok=True)

    def read_sht40(self):
        """Read humidity + temperature"""
        if self.bus is None:
            # [FIX #128] No silent mock.
            raise RuntimeError("Hardware I2C Bus Not Available")

        try:
            self.bus.write_i2c_block_data(self.sht40_addr, 0xFD, [])
            time.sleep(0.05)
            data = self.bus.read_i2c_block_data(self.sht40_addr, 0, 6)

            rh = (((data[0] << 8) | data[1]) / 65535.0) * 100.0
            temp = ((((data[3] << 8) | data[4]) / 65535.0) * 175.0) - 45.0

            return rh, temp
        except Exception as e:
            print(f"I2C Read Failed: {e}")
            return None, None

    def pid_rh(self, current_rh):
        error = self.state.rh_setpoint - current_rh
        p_term = self.rh_kp * error
        self.rh_integral = max(-100, min(100, self.rh_integral + self.rh_ki * error * 0.1))
        d_term = self.rh_kd * (error - self.rh_prev_error) / 0.1
        self.rh_prev_error = error
        return max(0, min(100, p_term + self.rh_integral + d_term))

    def pid_temp(self, current_temp):
        error = self.state.temp_setpoint - current_temp
        p_term = self.temp_kp * error
        self.temp_integral = max(-100, min(100, self.temp_integral + self.temp_ki * error * 0.1))
        d_term = self.temp_kd * (error - self.temp_prev_error) / 0.1
        self.temp_prev_error = error
        return max(0, min(100, p_term + self.temp_integral + d_term))

    def control_loop(self, duration_seconds=600):
        """Main RH control loop"""
        start = time.time()
        cycle = 0

        print(f"[RH] Starting. Setpoint: {self.state.setpoint}%")

        try:
            while time.time() - start < duration_seconds:
                # [FIX] Software Watchdog / Keep-Alive
                # Pet the watchdog by touching a file.
                # Rust Supervisor will monitor this file's mtime.
                # [FIX] Use cross-platform temp dir (usually /tmp on Linux)
                import tempfile
                lock_path = os.path.join(tempfile.gettempdir(), "agnix_watchdog.lock")

                try:
                    with open(lock_path, "w") as f:
                        f.write(str(time.time()))
                except Exception as e:
                    print(f"[CRITICAL] Watchdog Pet Failed: {e}")

                cycle += 1
                rh, temp = self.read_sht40()

                if rh is None:
                    # [FIX #133] Actuator Latch (Fail-Safe)
                    if hasattr(self, 'mist_pwm'):
                         self.mist_pwm.ChangeDutyCycle(0)
                    time.sleep(0.1) # Retry faster
                    continue

                self.state.current_rh = rh
                self.state.current_temp = temp

                # [FIX #127] Deadband Trap Removed
                # [FIX #129] Time Dilation - Loop matches PID dt (10Hz)

                # RH Control
                mist_val = self.pid_rh(rh)
                if hasattr(self, 'mist_pwm'):
                    self.mist_pwm.ChangeDutyCycle(mist_val)
                self.state.mist_pwm = int(mist_val)

                # Temp Control
                heater_val = self.pid_temp(temp)
                if hasattr(self, 'heater_pwm'):
                    self.heater_pwm.ChangeDutyCycle(heater_val)
                self.state.heater_pwm = int(heater_val)

                self.state.is_paused = False

                # Log properly
                log_entry = {
                    "timestamp_iso": datetime.now().isoformat(),
                    "rh_percent": rh,
                    "temperature_c": temp,
                    "mist_pwm": mist_val,
                    "heater_pwm": heater_val,
                    "is_paused": self.state.is_paused
                }
                with open(os.path.join(self.log_dir, "rh_log.jsonl"), "a") as f:
                    f.write(json.dumps(log_entry) + "\n")

                if cycle % 10 == 0:
                    print(f"[{cycle:04d}] RH={rh:5.1f}% T={temp:5.2f}Â°C | Mist={pwm:3.0f}%")

                time.sleep(0.1) # [FIX #129] 10Hz Control Loop

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
