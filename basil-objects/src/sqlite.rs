use basil_common::{Result, BasilError};
use basil_bytecode::Value;
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

// Global connection table guarded by a Mutex, initialized on first use.
static CONNS: OnceLock<Mutex<ConnTable>> = OnceLock::new();
fn conns() -> &'static Mutex<ConnTable> { CONNS.get_or_init(|| Mutex::new(ConnTable::default())) }

#[derive(Default)]
struct ConnTable {
    next: i32,
    map: HashMap<i32, Connection>,
}

impl ConnTable {
    fn insert(&mut self, conn: Connection) -> i32 {
        self.next = self.next.saturating_add(1);
        let id = self.next;
        self.map.insert(id, conn);
        id
    }
    fn get_mut(&mut self, id: i32) -> Option<&mut Connection> { self.map.get_mut(&id) }
    fn remove(&mut self, id: i32) -> bool { self.map.remove(&id).is_some() }
}

pub fn register(_reg: &mut crate::Registry) {
    // No object types; built-ins are wired in VM when feature is enabled.
}

pub fn sqlite_open(path: &str) -> i64 {
    match Connection::open(path) {
        Ok(conn) => {
            let mut tbl = conns().lock().unwrap();
            let id = tbl.insert(conn);
            id as i64
        }
        Err(_) => 0,
    }
}

pub fn sqlite_close(handle: i64) {
    let mut tbl = conns().lock().unwrap();
    let _ = tbl.remove(handle as i32);
}

pub fn sqlite_exec(handle: i64, sql: &str) -> i64 {
    let mut tbl = conns().lock().unwrap();
    let conn = match tbl.get_mut(handle as i32) { Some(c)=>c, None=> return -1 };
    match conn.execute(sql, []) { Ok(n)=> n as i64, Err(_)=> -1 }
}

pub fn sqlite_query2d(handle: i64, sql: &str) -> Result<Value> {
    let mut tbl = conns().lock().unwrap();
    let conn = tbl.get_mut(handle as i32).ok_or_else(|| BasilError("SQLITE_QUERY2D$: invalid handle".into()))?;

    let mut stmt = conn.prepare(sql)
        .map_err(|e| BasilError(format!("SQLITE_QUERY2D$: prepare failed: {}", e)))?;

    let col_cnt = stmt.column_count() as usize;

    let rows_iter = stmt.query_map([], |row| {
        let mut out: Vec<String> = Vec::with_capacity(col_cnt);
        for i in 0..col_cnt {
            let v: rusqlite::types::Value = row.get::<usize, rusqlite::types::Value>(i)?;
            let s = match v {
                rusqlite::types::Value::Null => String::new(),
                rusqlite::types::Value::Integer(n) => n.to_string(),
                rusqlite::types::Value::Real(f) => {
                    let mut s = f.to_string();
                    if s.ends_with(".0") { s.truncate(s.len()-2); }
                    s
                }
                rusqlite::types::Value::Text(t) => t, 
                rusqlite::types::Value::Blob(_b) => {
                    // Avoid adding base64 dep here; simple placeholder
                    String::from("[BLOB]")
                }
            };
            out.push(s);
        }
        Ok(out)
    }).map_err(|e| BasilError(format!("SQLITE_QUERY2D$: query failed: {}", e)))?;

    let mut data: Vec<String> = Vec::new();
    let mut row_cnt = 0usize;
    for r in rows_iter {
        let row = r.map_err(|e| BasilError(format!("SQLITE_QUERY2D$: row read failed: {}", e)))?;
        if row.len() != col_cnt {
            return Err(BasilError("SQLITE_QUERY2D$: inconsistent column count".into()));
        }
        data.extend(row);
        row_cnt += 1;
    }

    Ok(Value::StrArray2D { rows: row_cnt, cols: col_cnt, data })
}

pub fn sqlite_last_insert_id(handle: i64) -> i64 {
    let mut tbl = conns().lock().unwrap();
    if let Some(conn) = tbl.get_mut(handle as i32) {
        conn.last_insert_rowid() as i64
    } else { 0 }
}
