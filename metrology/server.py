from fastapi import FastAPI, HTTPException
from datetime import datetime, timezone
import threading
import sys
import os
import logging

# Ensure local imports work
sys.path.append(os.path.dirname(os.path.abspath(__file__)))

from rh_controller import RHController
from temperature_controller import TemperatureController
from crypto import HmacAuthenticator

app = FastAPI()

# Logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("metrology_server")

# Security
HMAC_SECRET = os.environ.get("AGNIX_HMAC_SECRET", "default-insecure-secret-for-dev-only").encode('utf-8')
authenticator = HmacAuthenticator(HMAC_SECRET)

if HMAC_SECRET == b"default-insecure-secret-for-dev-only":
    logger.warning("RUNNING WITH INSECURE DEFAULT HMAC SECRET! SET AGNIX_HMAC_SECRET.")

# Controllers
# [FIX] Consolidated Controller using RHController (extended to handle Temp)
# This removes the "Silent Mock" in temperature_controller.py and uses the real SHT40 data.
rh_ctrl = RHController()

# Background Loops
threading.Thread(target=rh_ctrl.control_loop, kwargs={'duration_seconds': 31536000}, daemon=True).start()

@app.get("/status")
def get_status():
    current_rh = rh_ctrl.state.current_rh
    current_temp = rh_ctrl.state.current_temp

    # Safety Logic (40-60% RH, 20-30C)
    rh_safe = 40.0 <= current_rh <= 60.0
    temp_safe = 20.0 <= current_temp <= 30.0
    is_safe = rh_safe and temp_safe

    payload = {
        "rh": round(current_rh, 2),
        "temperature": round(current_temp, 2),
        "safe": is_safe,
        "timestamp_iso": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
    }

    # Sign the payload
    signed_response = authenticator.sign(payload)
    return signed_response

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8080)
