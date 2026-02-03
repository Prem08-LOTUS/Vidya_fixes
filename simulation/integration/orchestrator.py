#!/usr/bin/env python3
"""
MASTER ORCHESTRATOR
Integrates all simulation modules (environment, mechanical, measurement)
Synchronizes with fixed time-step
Status: PRODUCTION-READY âœ…
"""

import sys
import os
sys.path.insert(0, os.path.expanduser("~/agni-workspace/simulation"))

from environment.climate_controller import ClimateController
from mechanical.xy_stage import XYStageSimulator
from measurement.sensor_simulator import MeasurementSimulator
import json
from datetime import datetime

class MicroserviceOrchestrator:
    """Orchestrates all simulation modules"""

    def __init__(self):
        self.climate = ClimateController()
        self.stage = XYStageSimulator()
        self.sensors = MeasurementSimulator()

        self.dt = 0.01  # 10ms time-step
        self.cycle = 0
        self.status = "INITIALIZED"

        print("[ORCHESTRATOR] Initialized")

    def step(self) -> dict:
        """Execute one integrated simulation step"""
        self.cycle += 1

        # 1. Climate control step
        climate_state = self.climate.step()

        # 2. Mechanical stage step
        stage_state = self.stage.step(dt=self.dt)

        # 3. Measurement step (use stage Z as true position)
        meas_state = self.sensors.measure(true_z_nm=stage_state.z_um * 1000)  # Convert Âµm to nm

        # 4. Create unified log entry
        log_entry = {
            "cycle": self.cycle,
            "timestamp": datetime.now().isoformat(),
            "climate": {
                "temperature_c": round(climate_state.temperature_c, 2),
                "humidity_pct": round(climate_state.humidity_pct, 2),
                "heater_pwm": round(climate_state.heater_pwm, 1),
                "mist_pwm": round(climate_state.mist_pwm, 1),
            },
            "stage": {
                "x_um": round(stage_state.x_um, 2),
                "y_um": round(stage_state.y_um, 2),
                "z_um": round(stage_state.z_um, 2),
            },
            "sensors": {
                "z_cap_nm": round(meas_state.capacitive_z_noisy, 2),
                "i_tunnel_pa": round(meas_state.tunneling_i_pa, 3),
                "z_fused_nm": round(meas_state.fused_z_nm, 2),
            },
        }

        return log_entry

    def run_scenario(self, duration_cycles: int = 1000, log_interval: int = 100):
        """Run full integrated simulation"""
        print(f"\n[ORCHESTRATOR] Starting {duration_cycles}-cycle scenario")
        print(f"  Environment: T_setpoint=25Â°C, RH_setpoint=50%")
        print(f"  Stage: Moving XY plane")
        print(f"  Sensors: Capacitive + Tunneling + Fusion")
        print()

        log_file = os.path.expanduser("~/agni-workspace/results/integrated_sim.jsonl")
        os.makedirs(os.path.dirname(log_file), exist_ok=True)

        for i in range(duration_cycles):
            # Execute step
            log_entry = self.step()

            # Log to file
            with open(log_file, "a") as f:
                f.write(json.dumps(log_entry) + "\n")

            # Print progress
            if i % log_interval == 0:
                print(
                    f"[{i:05d}] T={log_entry['climate']['temperature_c']:.1f}Â°C "
                    f"RH={log_entry['climate']['humidity_pct']:.1f}% | "
                    f"Z_fused={log_entry['sensors']['z_fused_nm']:.1f} nm"
                )

        print(f"\n[ORCHESTRATOR] Scenario complete. Results: {log_file}")
        return log_file

    def cleanup(self):
        """Cleanup resources"""
        self.stage.disconnect()
        print("[ORCHESTRATOR] Cleanup complete")

if __name__ == "__main__":
    orch = MicroserviceOrchestrator()
    orch.run_scenario(duration_cycles=1000, log_interval=100)
    orch.cleanup()
