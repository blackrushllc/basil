#[cfg(feature = "db_sqlite")]
use rusqlite::{params, Connection};
use crate::models::MinerSnapshot;

pub struct SqliteLogger {
    path: String,
}

impl SqliteLogger {
    pub fn new(path: String) -> Self {
        Self { path }
    }

    #[cfg(feature = "db_sqlite")]
    pub fn init(&self) -> Result<(), String> {
        let conn = Connection::open(&self.path).map_err(|e| e.to_string())?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS snapshots (
                id INTEGER PRIMARY KEY,
                ts INTEGER,
                data_json TEXT,
                db_ok INTEGER
            )",
            [],
        ).map_err(|e| e.to_string())?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS alerts (
                id INTEGER PRIMARY KEY,
                snapshot_id INTEGER,
                message TEXT
            )",
            [],
        ).map_err(|e| e.to_string())?;
        
        Ok(())
    }

    #[cfg(feature = "db_sqlite")]
    pub fn write(&self, snapshot: &MinerSnapshot) -> Result<(), String> {
        let mut conn = Connection::open(&self.path).map_err(|e| e.to_string())?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        
        let data_json = serde_json::to_string(snapshot).map_err(|e| e.to_string())?;
        tx.execute(
            "INSERT INTO snapshots (ts, data_json, db_ok) VALUES (?, ?, ?)",
            params![snapshot.timestamp, data_json, if snapshot.db_ok { 1 } else { 0 }],
        ).map_err(|e| e.to_string())?;
        
        let snapshot_id = tx.last_insert_rowid();
        
        for alert in &snapshot.alerts {
            tx.execute(
                "INSERT INTO alerts (snapshot_id, message) VALUES (?, ?)",
                params![snapshot_id, alert],
            ).map_err(|e| e.to_string())?;
        }
        
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    #[cfg(not(feature = "db_sqlite"))]
    pub fn init(&self) -> Result<(), String> {
        Err("SQLite support not enabled".to_string())
    }

    #[cfg(not(feature = "db_sqlite"))]
    pub fn write(&self, _snapshot: &MinerSnapshot) -> Result<(), String> {
        Err("SQLite support not enabled".to_string())
    }
}
