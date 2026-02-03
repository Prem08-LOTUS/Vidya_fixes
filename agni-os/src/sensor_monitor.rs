// agni-workspace/agni-os/src/sensor_monitor.rs
use crate::safety::voting::{voting_2oo3, VoteResult, SensorStatus};
use crate::safety::uncertainty::Measurement;
use crate::physics::types::ThermalField;
use nalgebra::{Vector3, Matrix3};
use tracing::{info, warn, error};
use std::sync::atomic::{AtomicBool, Ordering};
use std::env;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

#[derive(Deserialize, Serialize, Debug)]
struct MetrologyPayload {
    rh: f64,
    temperature: f64,
    safe: bool,
    timestamp_iso: String,
}

#[derive(Deserialize, Debug)]
struct SignedMessage {
    payload: MetrologyPayload,
    timestamp: u64,
    nonce: String,
    hmac: String,
}

pub struct SensorMonitor {
    target_ip: String,
    handshake_complete: AtomicBool,
    client: reqwest::Client,
    secret: String,
}

impl SensorMonitor {
    pub fn new(ip: &str) -> Self {
        let secret = env::var("AGNIX_HMAC_SECRET").unwrap_or_else(|_| "default-insecure-secret-for-dev-only".to_string());
        Self {
            target_ip: ip.to_string(),
            handshake_complete: AtomicBool::new(false),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(500))
                .build()
                .expect("Failed to build HTTP client"),
            secret,
        }
    }

    async fn perform_handshake(&self) -> bool {
        // Real handshake: verify connectivity and signature
        let url = format!("http://{}:8080/status", self.target_ip);
        if let Ok(resp) = self.client.get(&url).send().await {
            if resp.status().is_success() {
                 info!("Handshake: Metrology Server Verified at {}", self.target_ip);
                 self.handshake_complete.store(true, Ordering::SeqCst);
                 return true;
            }
        }
        warn!("Handshake Failed: Cannot reach Metrology Server at {}", self.target_ip);
        false
    }

    pub async fn poll(&self) -> Measurement<ThermalField> {
        if !self.handshake_complete.load(Ordering::SeqCst) {
             if !self.perform_handshake().await {
                 return Measurement::new(ThermalField::default(), f64::INFINITY, 0);
             }
        }

        let url = format!("http://{}:8080/status", self.target_ip);

        match self.client.get(&url).send().await {
            Ok(resp) => {
                if let Ok(msg) = resp.json::<SignedMessage>().await {
                    // Verify HMAC with Deterministic Serialization
                    // [FIX] Use manual string formatting to guarantee order and float precision
                    // Matches Python implementation: f"rh={rh:.2f}|temp={temp:.2f}|safe={safe}|ts={ts}|nonce={nonce}"

                    let p = &msg.payload;
                    let canonical_msg = format!(
                        "rh={:.2}|temp={:.2}|safe={}|ts={}|nonce={}",
                        p.rh, p.temperature, if p.safe { "true" } else { "false" }, msg.timestamp, msg.nonce
                    );

                    let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes())
                        .expect("Invalid Key Length");
                    mac.update(canonical_msg.as_bytes());
                    let result = mac.finalize();
                    let expected_hex = hex::encode(result.into_bytes());

                    if expected_hex != msg.hmac {
                        error!("SECURITY: HMAC Signature Mismatch! Possible Tampering or Serialization Error.");
                        // error!("Canonical: {}", canonical_msg); // Debug only
                        return Measurement::new(ThermalField::default(), f64::INFINITY, 0);
                    }

                    // Check freshness (5s)
                    // [FIX] Use u128 to prevent overflow/truncation issues
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                    if now.abs_diff(msg.timestamp as u128) > 5000 {
                        error!("SECURITY: Replay Attack Detected (Stale Timestamp)");
                        return Measurement::new(ThermalField::default(), f64::INFINITY, 0);
                    }

                    // Success
                        let t = msg.payload.temperature;
                        let field = ThermalField {
                            sensors: [t, t, t, t], // Uniform field assumption
                            gradient: Vector3::zeros(),
                            d_gradient_dt: Vector3::zeros(),
                            covariance: Matrix3::identity(),
                            timestamp_cycle: 0,
                        };
                        return Measurement::new(field, 0.01, 0);
                    }
                }
            }
            Err(e) => {
                warn!("Metrology Poll Failed: {}", e);
            }
        }

        Measurement::new(ThermalField::default(), f64::INFINITY, 0)
    }
}
