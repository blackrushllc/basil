use crate::models::MinerSnapshot;

pub struct AlertRules {
    pub temp_max: Option<f64>,
    pub hashrate_min: Option<f64>,
    pub reject_rate_max: Option<f64>,
}

impl Default for AlertRules {
    fn default() -> Self {
        Self {
            temp_max: None,
            hashrate_min: None,
            reject_rate_max: None,
        }
    }
}

pub struct AlertEvaluator {
    pub rules: AlertRules,
}

impl AlertEvaluator {
    pub fn new() -> Self {
        Self {
            rules: AlertRules::default(),
        }
    }

    pub fn evaluate(&self, snapshot: &MinerSnapshot) -> Vec<String> {
        let mut alerts = Vec::new();
        for asic in &snapshot.asics {
            if !asic.online {
                alerts.push(format!("ASIC {} is offline", asic.name));
                continue;
            }
            if let Some(stats) = &asic.stats {
                if let Some(limit) = self.rules.temp_max {
                    if stats.temp_max_c > limit {
                        alerts.push(format!(
                            "ASIC {} temperature high: {:.1}C (limit {:.1}C)",
                            asic.name, stats.temp_max_c, limit
                        ));
                    }
                }
                if let Some(limit) = self.rules.hashrate_min {
                    if stats.hashrate_ghs < limit {
                        alerts.push(format!(
                            "ASIC {} hashrate low: {:.1} GH/s (limit {:.1} GH/s)",
                            asic.name, stats.hashrate_ghs, limit
                        ));
                    }
                }
                if let Some(limit) = self.rules.reject_rate_max {
                    if stats.reject_rate_pct > limit {
                        alerts.push(format!(
                            "ASIC {} reject rate high: {:.2}% (limit {:.2}%)",
                            asic.name, stats.reject_rate_pct, limit
                        ));
                    }
                }
            }
        }
        alerts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AsicSnapshot, MinerStats};

    #[test]
    fn test_alert_evaluator() {
        let mut evaluator = AlertEvaluator::new();
        evaluator.rules.temp_max = Some(80.0);
        evaluator.rules.hashrate_min = Some(100.0);

        let stats = MinerStats {
            model: "S19".into(),
            firmware: "1.0".into(),
            uptime_s: 100,
            hashrate_ghs: 90.0, // Low
            temp_max_c: 85.0,   // High
            fan_rpm_avg: 3000,
            accepted: 100,
            rejected: 1,
            reject_rate_pct: 1.0,
            pool_url: "pool".into(),
            pool_user: "user".into(),
            raw: serde_json::Value::Null,
            hashrate_ghs_5m: None,
            temp_avg_c: None,
            fan_rpm_min: None,
        };

        let snapshot = MinerSnapshot {
            timestamp: 12345678,
            asics: vec![
                AsicSnapshot {
                    name: "M1".into(),
                    host: "localhost".into(),
                    online: true,
                    stats: Some(stats),
                    last_err: None,
                },
                AsicSnapshot {
                    name: "M2".into(),
                    host: "localhost".into(),
                    online: false,
                    stats: None,
                    last_err: None,
                },
            ],
            alerts: vec![],
            db_ok: true,
            last_err: None,
        };

        let alerts = evaluator.evaluate(&snapshot);
        assert_eq!(alerts.len(), 3);
        assert!(alerts[0].contains("temperature high"));
        assert!(alerts[1].contains("hashrate low"));
        assert!(alerts[2].contains("is offline"));
    }
}
