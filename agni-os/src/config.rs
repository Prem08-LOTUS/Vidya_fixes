use serde::Deserialize;
use std::fs;
use anyhow::{Result, Context};

#[derive(Debug, Deserialize, Clone)]
pub struct PhysicsConfig {
    pub max_vel_nm_s: f64,
    pub max_acc_nm_s2: f64,
    pub soft_limit_min_nm: f64,
    pub soft_limit_max_nm: f64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SafetyConfig {
    pub min_safe_voltage: f64,
    pub metrology_trust_limit_nm: f64,
    pub physics: PhysicsConfig,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            min_safe_voltage: 22.0,
            metrology_trust_limit_nm: 5.0,
            physics: PhysicsConfig {
                max_vel_nm_s: 5000.0,
                max_acc_nm_s2: 50000.0,
                soft_limit_min_nm: -100.0,
                soft_limit_max_nm: 1000000.0,
            }
        }
    }
}

pub fn load_config(path: &str) -> Result<SafetyConfig> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path))?;

    let config: SafetyConfig = toml::from_str(&content)
        .with_context(|| "Failed to parse TOML config")?;

    Ok(config)
}
