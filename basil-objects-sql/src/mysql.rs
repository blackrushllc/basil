use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use basil_common::{BasilError, Result};
use basil_bytecode::{ArrayObj, BasicObject, ElemType, MethodDesc, ObjectDescriptor, PropDesc, Value};
use sqlx::Row;

use crate::runtime::TOKIO_RT;

fn make_str_array(items: Vec<String>) -> Value {
    let dims = vec![items.len()];
    let data = items.into_iter().map(Value::Str).collect::<Vec<_>>();
    let arr = Rc::new(ArrayObj { elem: ElemType::Str, dims, data: RefCell::new(data) });
    Value::Array(arr)
}

fn str_arg(v: &Value) -> String { match v { Value::Str(s)=>s.clone(), Value::Int(i)=>i.to_string(), Value::Num(n)=> n.to_string(), Value::Bool(b)=> b.to_string(), _=> String::new() } }

fn bad_arity(name: &str, expect: usize, got: usize) -> BasilError { BasilError(format!("DB_MYSQL.{}: expected {} args, got {}", name, expect, got)) }

fn err(op: &str, e: impl std::fmt::Display) -> BasilError { BasilError(format!("SQL(MySQL) {}: {}", op, e)) }

#[derive(Clone)]
pub struct MySqlObj {
    dsn: Option<String>,
    pool_max: u32,
    connect_timeout_ms: u64,
    command_timeout_ms: u64,
    tls_mode: String,
    root_cert_path: Option<String>,
    last_rows: i64,
    last_err: String,
    // pooled connector
    pool: Option<sqlx::MySqlPool>,
    // transaction connection if active
    txn_conn: Option<sqlx::pool::PoolConnection<sqlx::MySql>>,
}

impl Default for MySqlObj {
    fn default() -> Self {
        Self {
            dsn: None,
            pool_max: 5,
            connect_timeout_ms: 5000,
            command_timeout_ms: 30000,
            tls_mode: "REQUIRED".to_string(),
            root_cert_path: None,
            last_rows: 0,
            last_err: String::new(),
            pool: None,
            txn_conn: None,
        }
    }
}

impl MySqlObj {
    fn descriptor_static() -> ObjectDescriptor {
        ObjectDescriptor {
            type_name: "DB_MYSQL".into(),
            version: "0.1".into(),
            summary: "MySQL/Aurora connector with pooled connections (SQLx/rustls)".into(),
            properties: vec![
                PropDesc { name: "PoolMax%".into(), type_name: "Int".into(), readable: true, writable: true },
                PropDesc { name: "ConnectTimeoutMs%".into(), type_name: "Int".into(), readable: true, writable: true },
                PropDesc { name: "CommandTimeoutMs%".into(), type_name: "Int".into(), readable: true, writable: true },
                PropDesc { name: "TlsMode$".into(), type_name: "String".into(), readable: true, writable: true },
                PropDesc { name: "RootCertPath$".into(), type_name: "String".into(), readable: true, writable: true },
                PropDesc { name: "LastRowsAffected%".into(), type_name: "Int".into(), readable: true, writable: false },
                PropDesc { name: "LastError$".into(), type_name: "String".into(), readable: true, writable: false },
            ],
            methods: vec![
                MethodDesc { name: "Connect$".into(), arity: 6, arg_names: vec!["host$".into(), "port%".into(), "user$".into(), "pass$".into(), "dbname$".into(), "ssl-mode$".into()], return_type: "Int (ok%)".into() },
                MethodDesc { name: "Execute".into(), arity: 2, arg_names: vec!["sql$".into(), "params$[]?".into()], return_type: "Int (rows%)".into() },
                MethodDesc { name: "Query$".into(), arity: 2, arg_names: vec!["sql$".into(), "params$[]?".into()], return_type: "String (json$)".into() },
                MethodDesc { name: "QueryTable$".into(), arity: 2, arg_names: vec!["sql$".into(), "params$[]?".into()], return_type: "String[]".into() },
                MethodDesc { name: "Begin".into(), arity: 0, arg_names: vec![], return_type: "Int (ok%)".into() },
                MethodDesc { name: "Commit".into(), arity: 0, arg_names: vec![], return_type: "Int (ok%)".into() },
                MethodDesc { name: "Rollback".into(), arity: 0, arg_names: vec![], return_type: "Int (ok%)".into() },
            ],
            examples: vec![
                "DIM db@ AS DB_MYSQL(\"mysql://user:pass@host:3306/db?ssl-mode=REQUIRED\")".into()
            ],
        }
    }

    fn ensure_pool(&mut self) -> Result<sqlx::MySqlPool> {
        if let Some(p) = &self.pool { return Ok(p.clone()); }
        let dsn = self.dsn.clone().ok_or_else(|| BasilError("DB_MYSQL: no DSN configured; pass in constructor or call Connect$".into()))?;

        // Build options from DSN
        let mut opts: sqlx::mysql::MySqlConnectOptions = dsn.parse().map_err(|_e| BasilError("SQL(MySQL) ConnectFailed: bad DSN".into()))?;
        // TLS mode mapping
        let tls = self.tls_mode.to_ascii_uppercase();
        use sqlx::mysql::MySqlSslMode;
        let mode = match tls.as_str() {
            "DISABLED" => MySqlSslMode::Disabled,
            "PREFERRED" => MySqlSslMode::Preferred,
            _ => MySqlSslMode::Required,
        };
        opts = opts.ssl_mode(mode);
        if let Some(path) = &self.root_cert_path { if !path.is_empty() { opts = opts.ssl_ca(PathBuf::from(path)); } }

        let mut builder = sqlx::mysql::MySqlPoolOptions::new();
        builder = builder.max_connections(self.pool_max);
        builder = builder.acquire_timeout(Duration::from_millis(self.connect_timeout_ms));
        // Connect on runtime
        let pool = TOKIO_RT.block_on(async move { builder.connect_with(opts).await })
            .map_err(|e| BasilError(format!("SQL(MySQL) ConnectFailed: {}", e)))?;
        self.pool = Some(pool.clone());
        Ok(pool)
    }

    fn get_executor<'a>(&'a mut self) -> Result<ExecutorEither<'a>> {
        if self.txn_conn.is_some() {
            return Ok(ExecutorEither::Txn(self.txn_conn.as_mut().unwrap()));
        }
        let pool = self.ensure_pool()?;
        Ok(ExecutorEither::Pool(pool))
    }

    fn bind_mysql<'q>(mut qb: sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments>, params: &'q [String]) -> sqlx::query::Query<'q, sqlx::MySql, sqlx::mysql::MySqlArguments> {
        for p in params { qb = qb.bind(p); }
        qb
    }

    fn to_json_rows(rows: Vec<sqlx::mysql::MySqlRow>) -> String {
        use serde_json::{Value as J, Map};
        let mut out = Vec::with_capacity(rows.len());
        for row in rows.iter() {
            let mut m = Map::new();
            for col in row.columns() {
                let name = col.name().to_string();
                // Try common types; fall back to string
                let jv = if let Ok(v) = row.try_get::<i64, _>(name.as_str()) { J::from(v) }
                else if let Ok(v) = row.try_get::<f64, _>(name.as_str()) { J::from(v) }
                else if let Ok(v) = row.try_get::<bool, _>(name.as_str()) { J::from(v) }
                else if let Ok(v) = row.try_get::<String, _>(name.as_str()) { J::from(v) }
                else if let Ok(v) = row.try_get::<Vec<u8>, _>(name.as_str()) { J::from(base64::encode(v)) }
                else { J::Null };
                m.insert(name, jv);
            }
            out.push(J::Object(m));
        }
        serde_json::to_string(&out).unwrap_or_else(|_| "[]".to_string())
    }

    fn to_table_lines(rows: Vec<sqlx::mysql::MySqlRow>) -> Vec<String> {
        let mut lines = Vec::new();
        if rows.is_empty() { return lines; }
        // header
        let header = rows[0].columns().iter().map(|c| c.name().to_string()).collect::<Vec<_>>().join(",");
        lines.push(header);
        for row in rows.iter() {
            let mut vals: Vec<String> = Vec::new();
            for col in row.columns() {
                let name = col.name();
                let s = if let Ok(v) = row.try_get::<String, _>(name) { v }
                else if let Ok(v) = row.try_get::<i64, _>(name) { v.to_string() }
                else if let Ok(v) = row.try_get::<f64, _>(name) { let mut s=v.to_string(); if s.ends_with('.'){s.push('0');} s }
                else if let Ok(v) = row.try_get::<bool, _>(name) { if v {"1"} else {"0"}.to_string() }
                else if let Ok(v) = row.try_get::<Vec<u8>, _>(name) { format!("[{} bytes]", v.len()) }
                else { String::new() };
                vals.push(s);
            }
            lines.push(vals.join(","));
        }
        lines
    }

    fn parse_params(args: &[Value]) -> Vec<String> {
        if args.len()<2 { return Vec::new(); }
        match &args[1] {
            Value::Array(arr) => {
                let data = arr.data.borrow();
                data.iter().map(|v| match v { Value::Str(s)=>s.clone(), Value::Int(i)=>i.to_string(), Value::Num(n)=>n.to_string(), Value::Bool(b)=>b.to_string(), _=> String::new() }).collect()
            }
            _ => Vec::new()
        }
    }
}

enum ExecutorEither<'a> {
    Pool(sqlx::MySqlPool),
    Txn(&'a mut sqlx::pool::PoolConnection<sqlx::MySql>),
}

impl<'a> Executor<'a> for ExecutorEither<'a> {
    type Database = sqlx::MySql;
    fn fetch_many<'e, 'q: 'e, E>(self, query: E) -> <Self::Database as sqlx::database::HasExecutor<'e>>::FetchMany where E: sqlx::Execute<'q, Self::Database> { match self { ExecutorEither::Pool(p)=> p.fetch_many(query), ExecutorEither::Txn(c)=> c.fetch_many(query) } }
    fn fetch_optional<'e, 'q: 'e, E>(self, query: E) -> <Self::Database as sqlx::database::HasExecutor<'e>>::FetchOptional where E: sqlx::Execute<'q, Self::Database> { match self { ExecutorEither::Pool(p)=> p.fetch_optional(query), ExecutorEither::Txn(c)=> c.fetch_optional(query) } }
}

impl BasicObject for MySqlObj {
    fn type_name(&self) -> &str { "DB_MYSQL" }

    fn get_prop(&self, name: &str) -> Result<Value> {
        match name.to_ascii_uppercase().as_str() {
            "POOLMAX%" => Ok(Value::Int(self.pool_max as i64)),
            "CONNECTTIMEOUTMS%" => Ok(Value::Int(self.connect_timeout_ms as i64)),
            "COMMANDTIMEOUTMS%" => Ok(Value::Int(self.command_timeout_ms as i64)),
            "TLSMODE$" => Ok(Value::Str(self.tls_mode.clone())),
            "ROOTCERTPATH$" => Ok(Value::Str(self.root_cert_path.clone().unwrap_or_default())),
            "LASTROWSAFFECTED%" => Ok(Value::Int(self.last_rows)),
            "LASTERROR$" => Ok(Value::Str(self.last_err.clone())),
            _ => Err(BasilError("DB_MYSQL: unknown property".into())),
        }
    }

    fn set_prop(&mut self, name: &str, v: Value) -> Result<()> {
        match name.to_ascii_uppercase().as_str() {
            "POOLMAX%" => { self.pool_max = match v { Value::Int(i)=> i as u32, Value::Num(n)=> n as u32, _=> self.pool_max }; self.pool=None; Ok(()) }
            ,"CONNECTTIMEOUTMS%" => { self.connect_timeout_ms = match v { Value::Int(i)=> i as u64, Value::Num(n)=> n as u64, _=> self.connect_timeout_ms }; self.pool=None; Ok(()) }
            ,"COMMANDTIMEOUTMS%" => { self.command_timeout_ms = match v { Value::Int(i)=> i as u64, Value::Num(n)=> n as u64, _=> self.command_timeout_ms }; Ok(()) }
            ,"TLSMODE$" => { self.tls_mode = str_arg(&v).to_ascii_uppercase(); self.pool=None; Ok(()) }
            ,"ROOTCERTPATH$" => { let s = str_arg(&v); self.root_cert_path = if s.is_empty(){None}else{Some(s)}; self.pool=None; Ok(()) }
            ,_ => Err(BasilError("DB_MYSQL: unknown or read-only property".into()))
        }
    }

    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "CONNECT$" => {
                if args.len() != 6 { return Err(bad_arity("Connect$", 6, args.len())); }
                let host = str_arg(&args[0]);
                let port = match &args[1] { Value::Int(i)=> *i as u16, Value::Num(n)=> n.trunc() as u16, _=> 3306 };
                let user = str_arg(&args[2]);
                let pass = str_arg(&args[3]);
                let db = str_arg(&args[4]);
                let ssl = str_arg(&args[5]);
                let dsn = format!("mysql://{}:{}@{}:{}/{}?ssl-mode={}", user, pass, host, port, db, if ssl.is_empty(){"REQUIRED"} else {ssl});
                self.dsn = Some(dsn);
                self.pool = None;
                // try connect now
                let ok = self.ensure_pool().is_ok();
                Ok(Value::Int(if ok {1} else {0}))
            }
            ,"EXECUTE" => {
                if args.is_empty() { return Err(bad_arity("Execute", 1, args.len())); }
                let sql = str_arg(&args[0]);
                let params = Self::parse_params(args);
                self.last_err.clear();
                let timeout = self.command_timeout_ms;
                let exec_res: Result<u64> = (||{
                    let mut qb = sqlx::query(&sql);
                    qb = Self::bind_mysql(qb, &params);
                    match self.get_executor()? {
                        ExecutorEither::Pool(pool) => {
                            let fut = qb.execute(&pool);
                            let res = TOKIO_RT.block_on(async move { tokio::time::timeout(Duration::from_millis(timeout), fut).await })
                                .map_err(|_| err("Execute(Timeout)", "command timeout"))??;
                            Ok(res.rows_affected())
                        }
                        ExecutorEither::Txn(conn) => {
                            let fut = qb.execute(conn);
                            let res = TOKIO_RT.block_on(async move { tokio::time::timeout(Duration::from_millis(timeout), fut).await })
                                .map_err(|_| err("Execute(Timeout)", "command timeout"))??;
                            Ok(res.rows_affected())
                        }
                    }
                })();
                match exec_res { Ok(n)=> { self.last_rows = n as i64; Ok(Value::Int(self.last_rows)) }, Err(e)=> { self.last_err = format!("{}", e); Err(e) } }
            }
            ,"QUERY$" => {
                if args.is_empty() { return Err(bad_arity("Query$", 1, args.len())); }
                let sql = str_arg(&args[0]);
                let params = Self::parse_params(args);
                self.last_err.clear();
                let timeout = self.command_timeout_ms;
                let query_res: Result<String> = (||{
                    let mut qb = sqlx::query(&sql);
                    qb = Self::bind_mysql(qb, &params);
                    match self.get_executor()? {
                        ExecutorEither::Pool(pool) => {
                            let fut = qb.fetch_all(&pool);
                            let rows = TOKIO_RT.block_on(async move { tokio::time::timeout(Duration::from_millis(timeout), fut).await })
                                .map_err(|_| err("Query(Timeout)", "command timeout"))??;
                            Ok(Self::to_json_rows(rows))
                        }
                        ExecutorEither::Txn(conn) => {
                            let fut = qb.fetch_all(conn);
                            let rows = TOKIO_RT.block_on(async move { tokio::time::timeout(Duration::from_millis(timeout), fut).await })
                                .map_err(|_| err("Query(Timeout)", "command timeout"))??;
                            Ok(Self::to_json_rows(rows))
                        }
                    }
                })();
                match query_res { Ok(s)=> Ok(Value::Str(s)), Err(e)=> { self.last_err = format!("{}", e); Err(e) } }
            }
            ,"QUERYTABLE$" => {
                if args.is_empty() { return Err(bad_arity("QueryTable$", 1, args.len())); }
                let sql = str_arg(&args[0]);
                let params = Self::parse_params(args);
                self.last_err.clear();
                let timeout = self.command_timeout_ms;
                let query_res: Result<Vec<String>> = (||{
                    let mut qb = sqlx::query(&sql);
                    qb = Self::bind_mysql(qb, &params);
                    match self.get_executor()? {
                        ExecutorEither::Pool(pool) => {
                            let fut = qb.fetch_all(&pool);
                            let rows = TOKIO_RT.block_on(async move { tokio::time::timeout(Duration::from_millis(timeout), fut).await })
                                .map_err(|_| err("QueryTable(Timeout)", "command timeout"))??;
                            Ok(Self::to_table_lines(rows))
                        }
                        ExecutorEither::Txn(conn) => {
                            let fut = qb.fetch_all(conn);
                            let rows = TOKIO_RT.block_on(async move { tokio::time::timeout(Duration::from_millis(timeout), fut).await })
                                .map_err(|_| err("QueryTable(Timeout)", "command timeout"))??;
                            Ok(Self::to_table_lines(rows))
                        }
                    }
                })();
                match query_res { Ok(lines)=> Ok(make_str_array(lines)), Err(e)=> { self.last_err = format!("{}", e); Err(e) } }
            }
            ,"BEGIN" => {
                if self.txn_conn.is_some() { return Ok(Value::Int(1)); }
                let pool = self.ensure_pool()?;
                let mut conn = TOKIO_RT.block_on(async move { pool.acquire().await }).map_err(|e| err("Begin(Acquire)", e))?;
                // BEGIN
                TOKIO_RT.block_on(async { sqlx::query("BEGIN").execute(&mut conn).await }).map_err(|e| err("Begin", e))?;
                self.txn_conn = Some(conn);
                Ok(Value::Int(1))
            }
            ,"COMMIT" => {
                if let Some(mut conn) = self.txn_conn.take() {
                    let res = TOKIO_RT.block_on(async { sqlx::query("COMMIT").execute(&mut conn).await });
                    drop(conn);
                    match res { Ok(_)=> Ok(Value::Int(1)), Err(e)=> Err(err("Commit", e)) }
                } else { Ok(Value::Int(1)) }
            }
            ,"ROLLBACK" => {
                if let Some(mut conn) = self.txn_conn.take() {
                    let res = TOKIO_RT.block_on(async { sqlx::query("ROLLBACK").execute(&mut conn).await });
                    drop(conn);
                    match res { Ok(_)=> Ok(Value::Int(1)), Err(e)=> Err(err("Rollback", e)) }
                } else { Ok(Value::Int(1)) }
            }
            ,other => Err(BasilError(format!("Unknown method '{}' on DB_MYSQL", other)))
        }
    }

    fn descriptor(&self) -> ObjectDescriptor { Self::descriptor_static() }
}

pub fn register<F: FnMut(&str, crate::TypeInfo)>(reg: &mut F) {
    let factory = |args: &[Value]| -> Result<Rc<RefCell<dyn BasicObject>>> {
        let mut obj = MySqlObj::default();
        if args.len() == 1 { obj.dsn = Some(str_arg(&args[0])); }
        Ok(Rc::new(RefCell::new(obj)))
    };
    let descriptor = || MySqlObj::descriptor_static();
    let constants = || Vec::<(String, Value)>::new();
    reg("DB_MYSQL", crate::TypeInfo { factory, descriptor, constants });
}
