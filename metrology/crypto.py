#!/usr/bin/env python3
"""HMAC-SHA256 message authentication (RFC 2104)"""

import hmac
import hashlib
import time
import uuid
import json
from typing import Any, Dict, Optional

class HmacAuthenticator:
    def __init__(self, secret_key: bytes):
        """Initialize with a 32-byte secret (or longer)"""
        if len(secret_key) < 32:
            raise ValueError("Secret key must be at least 32 bytes")
        self.secret = secret_key

    def sign(self, payload: Dict[str, Any], nonce: Optional[str] = None) -> Dict[str, Any]:
        """Sign a message payload"""
        if nonce is None:
            nonce = str(uuid.uuid4())

        timestamp = int(time.time() * 1000)  # milliseconds

        # [FIX] Use Deterministic Format matching Rust
        # "rh={:.2}|temp={:.2}|safe={}|ts={}|nonce={}"
        # Note: Python bool str is 'True'/'False', Rust is 'true'/'false'. We use lower.

        safe_str = "true" if payload["safe"] else "false"
        message_str = f"rh={payload['rh']:.2f}|temp={payload['temperature']:.2f}|safe={safe_str}|ts={timestamp}|nonce={nonce}"

        signature = hmac.new(self.secret, message_str.encode('utf-8'), hashlib.sha256).digest()

        return {
            "payload": payload,
            "timestamp": timestamp,
            "nonce": nonce,
            "hmac": signature.hex()
        }

    def verify(self, signed_msg: Dict[str, Any], max_age_ms: int = 5000) -> Optional[Dict[str, Any]]:
        """Verify and extract payload"""
        try:
            payload = signed_msg["payload"]
            timestamp = signed_msg["timestamp"]
            nonce = signed_msg["nonce"]
            hmac_hex = signed_msg["hmac"]

            # Freshness check
            now_ms = int(time.time() * 1000)
            if now_ms - timestamp > max_age_ms:
                return None  # Stale/replay

            if now_ms - timestamp < -1000:
                return None  # Future (clock skew)

            # Verify signature
            payload_json = json.dumps(payload, sort_keys=True, separators=(",", ":"))
            message = f"{payload_json}|{timestamp}|{nonce}".encode('utf-8')
            expected_sig = hmac.new(self.secret, message, hashlib.sha256).digest()

            # Constant-time comparison
            hmac_bytes = bytes.fromhex(hmac_hex)
            if len(hmac_bytes) != len(expected_sig):
                return None  # Length mismatch

            if not hmac.compare_digest(hmac_bytes, expected_sig):
                return None  # Signature invalid

            return payload

        except (KeyError, ValueError, TypeError):
            return None

# Usage
if __name__ == "__main__":
    secret = b"agni-secret-key-2025-minimum32bytes"
    auth = HmacAuthenticator(secret)

    payload = {"temperature_c": 25.0, "humidity_pct": 50.0}
    signed = auth.sign(payload)
    print("Signed:", signed)

    verified = auth.verify(signed)
    print("Verified:", verified)

    # Attack: tamper with payload
    signed["payload"]["temperature_c"] = 1000.0
    tampered = auth.verify(signed)
    print("Tampered (should be None):", tampered)
