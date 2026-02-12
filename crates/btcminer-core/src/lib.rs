pub mod models;
pub mod adapter;
pub mod profile;
pub mod generic_http;
pub mod alerts;
pub mod logger;

use std::collections::HashMap;
use crate::adapter::AsicAdapter;
use crate::models::{MinerSnapshot, AsicSnapshot};
use crate::profile::ProfileConfig;
use crate::generic_http::GenericHttpAdapter;
use crate::alerts::AlertEvaluator;
use chrono::Utc;

pub struct MinerManager {
    asics: HashMap<String, Box<dyn AsicAdapter>>,
    alerts: AlertEvaluator,
    last_snapshot: Option<MinerSnapshot>,
    last_err: Option<String>,
}

impl MinerManager {
    pub fn new() -> Self {
        Self {
            asics: HashMap::new(),
            alerts: AlertEvaluator::new(),
            last_snapshot: None,
            last_err: None,
        }
    }

    pub fn add_asic(&mut self, name: String, kind: String, host: String, user: String, pass: String, profile: Option<ProfileConfig>) -> Result<(), String> {
        if kind == "generic-http-json" {
            let prof = profile.ok_or_else(|| "Profile required for generic-http-json".to_string())?;
            let adapter = GenericHttpAdapter::new(host, user, pass, prof);
            self.asics.insert(name, Box::new(adapter));
            Ok(())
        } else {
            Err(format!("Unsupported kind: {}", kind))
        }
    }

    pub fn remove_asic(&mut self, name: &str) -> bool {
        self.asics.remove(name).is_some()
    }

    pub fn take_snapshot(&mut self) -> MinerSnapshot {
        let mut asic_snapshots = Vec::new();
        
        for (name, adapter) in &self.asics {
            let stats_res = adapter.stats();
            let asic_snap = match stats_res {
                Ok(stats) => AsicSnapshot {
                    name: name.clone(),
                    host: "".to_string(), // Adapter doesn't expose host easily, could add it to trait
                    online: true,
                    stats: Some(stats),
                    last_err: None,
                },
                Err(e) => AsicSnapshot {
                    name: name.clone(),
                    host: "".to_string(),
                    online: false,
                    stats: None,
                    last_err: Some(e),
                }
            };
            asic_snapshots.push(asic_snap);
        }

        let mut snapshot = MinerSnapshot {
            timestamp: Utc::now().timestamp(),
            asics: asic_snapshots,
            alerts: Vec::new(),
            db_ok: true,
            last_err: None,
        };

        snapshot.alerts = self.alerts.evaluate(&snapshot);
        self.last_snapshot = Some(snapshot.clone());
        snapshot
    }

    pub fn alerts_mut(&mut self) -> &mut AlertEvaluator {
        &mut self.alerts
    }
    
    pub fn last_err(&self) -> Option<String> {
        self.last_err.clone()
    }
}
