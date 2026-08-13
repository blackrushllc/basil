use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub profile_name: String,
    pub kind: String,
    pub auth: AuthConfig,
    pub endpoints: EndpointsConfig,
    pub extract: ExtractConfig,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    #[serde(rename = "type")]
    pub auth_type: String, // "basic", "cookie", "none"
    pub username_key: Option<String>,
    pub password_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointsConfig {
    pub info: Endpoint,
    pub stats: Endpoint,
    pub reboot: Option<Endpoint>,
    pub set_pools: Option<Endpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endpoint {
    pub method: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractConfig {
    pub model: String,
    pub firmware: String,
    pub uptime_s: String,
    pub hashrate_ghs: String,
    pub temp_max_c: String,
    pub fan_rpm_avg: String,
    pub accepted: String,
    pub rejected: String,
    pub pool_url: String,
    pub pool_user: String,
}

// Actually better extractor that handles [idx] more robustly
pub fn resolve_path<'a>(data: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = data;
    let parts: Vec<&str> = path.split('.').collect();

    for part in parts {
        if part.is_empty() {
            continue;
        }

        // Handle array access like "boards[0]"
        if let Some(bracket_pos) = part.find('[') {
            let name = &part[..bracket_pos];
            if !name.is_empty() {
                current = current.get(name)?;
            }

            let mut remaining = &part[bracket_pos..];
            while let Some(start) = remaining.find('[') {
                let end = remaining.find(']')?;
                let idx_str = &remaining[start + 1..end];
                let idx = idx_str.parse::<usize>().ok()?;
                current = current.get(idx)?;
                remaining = &remaining[end + 1..];
            }
        } else {
            current = current.get(part)?;
        }
    }

    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resolve_path() {
        let data = json!({
            "stats": {
                "uptime": 3600,
                "pools": [
                    { "url": "stratum+tcp://pool.com", "user": "worker1" }
                ]
            },
            "system": { "model": "S19" }
        });

        assert_eq!(
            resolve_path(&data, "stats.uptime").unwrap().as_i64(),
            Some(3600)
        );
        assert_eq!(
            resolve_path(&data, "stats.pools[0].url").unwrap().as_str(),
            Some("stratum+tcp://pool.com")
        );
        assert_eq!(
            resolve_path(&data, "system.model").unwrap().as_str(),
            Some("S19")
        );
        assert!(resolve_path(&data, "nonexistent").is_none());
    }
}
