use crate::adapter::AsicAdapter;
use crate::models::{MinerInfo, MinerStats, PoolConfig};
use crate::profile::{ProfileConfig, resolve_path};
use reqwest::blocking::Client;
use std::collections::HashMap;

#[derive(Debug)]
pub struct GenericHttpAdapter {
    pub host: String,
    pub user: String,
    pub pass: String,
    pub profile: ProfileConfig,
    pub client: Client,
}

impl GenericHttpAdapter {
    pub fn new(host: String, user: String, pass: String, profile: ProfileConfig) -> Self {
        Self {
            host,
            user,
            pass,
            profile,
            client: Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .unwrap(),
        }
    }

    fn get_url(&self, path: &str) -> String {
        format!("http://{}{}", self.host, path)
    }

    fn request(&self, method: &str, path: &str) -> Result<serde_json::Value, String> {
        let url = self.get_url(path);
        let mut req = match method {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url),
            _ => return Err(format!("Unsupported method: {}", method)),
        };

        if self.profile.auth.auth_type == "basic" {
            req = req.basic_auth(&self.user, Some(&self.pass));
        }

        let resp = req.send().map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HTTP error: {}", resp.status()));
        }

        resp.json().map_err(|e| e.to_string())
    }

    fn extract_string(&self, data: &serde_json::Value, path: &str) -> String {
        resolve_path(data, path)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    }

    fn extract_f64(&self, data: &serde_json::Value, path: &str) -> f64 {
        resolve_path(data, path)
            .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
            .unwrap_or(0.0)
    }

    fn extract_i64(&self, data: &serde_json::Value, path: &str) -> i64 {
        resolve_path(data, path)
            .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
            .unwrap_or(0)
    }
}

impl AsicAdapter for GenericHttpAdapter {
    fn info(&self) -> Result<MinerInfo, String> {
        let data = self.request(&self.profile.endpoints.info.method, &self.profile.endpoints.info.path)?;
        Ok(MinerInfo {
            model: self.extract_string(&data, &self.profile.extract.model),
            firmware: self.extract_string(&data, &self.profile.extract.firmware),
            hardware: "".to_string(),
            network: HashMap::new(),
        })
    }

    fn stats(&self) -> Result<MinerStats, String> {
        let data = self.request(&self.profile.endpoints.stats.method, &self.profile.endpoints.stats.path)?;
        
        let accepted = self.extract_i64(&data, &self.profile.extract.accepted);
        let rejected = self.extract_i64(&data, &self.profile.extract.rejected);
        let reject_rate_pct = if accepted + rejected > 0 {
            (rejected as f64 / (accepted + rejected) as f64) * 100.0
        } else {
            0.0
        };

        Ok(MinerStats {
            model: self.extract_string(&data, &self.profile.extract.model),
            firmware: self.extract_string(&data, &self.profile.extract.firmware),
            uptime_s: self.extract_i64(&data, &self.profile.extract.uptime_s),
            hashrate_ghs: self.extract_f64(&data, &self.profile.extract.hashrate_ghs),
            hashrate_ghs_5m: None,
            temp_max_c: self.extract_f64(&data, &self.profile.extract.temp_max_c),
            temp_avg_c: None,
            fan_rpm_avg: self.extract_i64(&data, &self.profile.extract.fan_rpm_avg),
            fan_rpm_min: None,
            accepted,
            rejected,
            reject_rate_pct,
            pool_url: self.extract_string(&data, &self.profile.extract.pool_url),
            pool_user: self.extract_string(&data, &self.profile.extract.pool_user),
            raw: data,
        })
    }

    fn set_pools(&self, _pools: &[PoolConfig]) -> Result<(), String> {
        Err("set_pools not implemented for generic-http-json yet".to_string())
    }

    fn reboot(&self) -> Result<(), String> {
        if let Some(endpoint) = &self.profile.endpoints.reboot {
            self.request(&endpoint.method, &endpoint.path)?;
            Ok(())
        } else {
            Err("reboot not supported by this profile".to_string())
        }
    }
}
