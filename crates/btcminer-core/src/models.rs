use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerStats {
    pub model: String,
    pub firmware: String,
    pub uptime_s: i64,
    pub hashrate_ghs: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hashrate_ghs_5m: Option<f64>,
    pub temp_max_c: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temp_avg_c: Option<f64>,
    pub fan_rpm_avg: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fan_rpm_min: Option<i64>,
    pub accepted: i64,
    pub rejected: i64,
    pub reject_rate_pct: f64,
    pub pool_url: String,
    pub pool_user: String,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerInfo {
    pub model: String,
    pub firmware: String,
    pub hardware: String,
    pub network: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    pub url: String,
    pub user: String,
    pub pass: String,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsicSnapshot {
    pub name: String,
    pub host: String,
    pub online: bool,
    pub stats: Option<MinerStats>,
    pub last_err: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinerSnapshot {
    pub timestamp: i64,
    pub asics: Vec<AsicSnapshot>,
    pub alerts: Vec<String>,
    pub db_ok: bool,
    pub last_err: Option<String>,
}
