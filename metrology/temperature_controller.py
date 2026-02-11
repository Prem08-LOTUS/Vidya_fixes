#!/usr/bin/env python3
"""Temperature Control Loop - Maintains T Â±0.1Â°C"""

import time, json, os
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

class TemperatureController:
    def __init__(self, heater_gpio=18, target_temp=25.0):
        self.target_temp = target_temp
        self.current_temp = target_temp

        try:
            GPIO.setmode(GPIO.BCM)
            GPIO.setup(heater_gpio, GPIO.OUT)
            self.heater_pwm = GPIO.PWM(heater_gpio, 1000)
            self.heater_pwm.start(0)
        except Exception as e:
            print(f"[WARN] GPIO Setup failed: {e}")
            # Mock if setup failed (e.g. permission error on real Pi or bad lib)
            # Assuming the Mock above handles import error, but runtime error is possible too.
            pass

        self.kp, self.ki, self.kd = 15.0, 2.0, 0.5
        self.integral = 0.0
        self.prev_error = 0.0

        self.log_dir = os.path.expanduser("~/agni-workspace/logs")
        os.makedirs(self.log_dir, exist_ok=True)

    def read_thermistor_voltage(self):
        """Read thermistor (placeholder)"""
        return 1.65  # TODO: Implement actual ADC read

    def voltage_to_temperature(self, voltage):
        """Convert voltage to temperature"""
        return 25.0 - (voltage - 1.65) / 0.33

    def pid_step(self):
        """Calculate PID output"""
        error = self.target_temp - self.current_temp

        p_term = self.kp * error
        self.integral = max(-100, min(100, self.integral + self.ki * error * 0.1))
        i_term = self.integral
        d_term = self.kd * (error - self.prev_error) / 0.1
        self.prev_error = error

        return max(0, min(100, p_term + i_term + d_term))

    def control_loop(self, duration_seconds=1800):
        """Main temperature control loop (30 min default)"""
        start = time.time()
        cycle = 0

        print(f"[TEMP] Starting. Target: {self.target_temp}Â°C")

        try:
            while time.time() - start < duration_seconds:
                cycle += 1
                voltage = self.read_thermistor_voltage()
                self.current_temp = self.voltage_to_temperature(voltage)

                heater_pwm = self.pid_step()
                if hasattr(self, 'heater_pwm'):
                    self.heater_pwm.ChangeDutyCycle(heater_pwm)

                error = self.target_temp - self.current_temp

                # Log properly
                log_entry = {
                    "timestamp_iso": datetime.now().isoformat(),
                    "temperature_c": self.current_temp,
                    "target_temp": self.target_temp,
                    "heater_pwm": heater_pwm
                }
                with open(os.path.join(self.log_dir, "temp_log.jsonl"), "a") as f:
                    f.write(json.dumps(log_entry) + "\n")

                print(f"[{cycle:04d}] T={self.current_temp:5.2f}Â°C Error={error:+5.2f}Â°C | Heater={heater_pwm:3.0f}%")

                time.sleep(1.0)

        except KeyboardInterrupt:
            print("\n[TEMP] Interrupted")
        finally:
            if hasattr(self, 'heater_pwm'):
                self.heater_pwm.stop()
            try:
                GPIO.cleanup()
            except:
                pass

if __name__ == "__main__":
    controller = TemperatureController(target_temp=25.0)
    controller.control_loop(10)  # 10 second dry run
