use basil_common::{Result, BasilError};
use basil_bytecode::{Value, ObjectDescriptor, MethodDesc, BasicObject};
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;
use btcminer_core::MinerManager;
use btcminer_core::models::MinerSnapshot;
use btcminer_core::profile::ProfileConfig;
use btcminer_core::generic_http::GenericHttpAdapter;
use btcminer_core::logger::SqliteLogger;
use btcminer_core::alerts::AlertEvaluator;

pub struct TypeInfo {
    pub factory: fn(args: &[Value]) -> Result<basil_bytecode::ObjectRef>,
    pub descriptor: fn() -> ObjectDescriptor,
    pub constants: fn() -> Vec<(String, Value)>,
}

pub fn register(add_type: &mut dyn FnMut(&str, TypeInfo)) {
    add_type("BTC_MINER_MGR", TypeInfo {
        factory: |_args| {
            Ok(Rc::new(RefCell::new(BtcMinerMgr {
                mgr: MinerManager::new(),
            })))
        },
        descriptor: || ObjectDescriptor {
            type_name: "BTC_MINER_MGR".into(),
            version: "1.0".into(),
            summary: "Bitcoin Miner Manager".into(),
            properties: vec![],
            methods: vec![
                MethodDesc { name: "ADD_ASIC%".into(), arity: 5, arg_names: vec!["name$".into(), "kind$".into(), "host$".into(), "user$".into(), "pass$".into()], return_type: "Integer".into() },
                MethodDesc { name: "REMOVE_ASIC%".into(), arity: 1, arg_names: vec!["name$".into()], return_type: "Integer".into() },
                MethodDesc { name: "SNAPSHOT@".into(), arity: 0, arg_names: vec![], return_type: "Object".into() },
                MethodDesc { name: "PROFILES@".into(), arity: 0, arg_names: vec![], return_type: "List".into() },
            ],
            examples: vec![],
        },
        constants: || Vec::new(),
    });

    add_type("BTC_ASIC", TypeInfo {
        factory: |args| {
            if args.len() < 4 { return Err(BasilError("BTC_ASIC expects 4 args (kind$, host$, user$, pass$)".into())); }
            let kind = args[0].to_string();
            let host = args[1].to_string();
            let user = args[2].to_string();
            let pass = args[3].to_string();
            
            let profile = if kind == "generic-http-json" {
                 let prof_json = include_str!("../../btcminer-core/profiles/antminer_http_v1.json");
                 serde_json::from_str::<ProfileConfig>(prof_json).map_err(|e| BasilError(e.to_string()))?
            } else {
                 return Err(BasilError(format!("Unsupported kind: {}", kind)));
            };

            let adapter = GenericHttpAdapter::new(host, user, pass, profile);
            Ok(Rc::new(RefCell::new(BtcAsic {
                adapter: Box::new(adapter),
                last_err: None,
            })))
        },
        descriptor: || ObjectDescriptor {
            type_name: "BTC_ASIC".into(),
            version: "1.0".into(),
            summary: "Individual Bitcoin ASIC".into(),
            properties: vec![],
            methods: vec![
                MethodDesc { name: "INFO@".into(), arity: 0, arg_names: vec![], return_type: "Object".into() },
                MethodDesc { name: "STATS@".into(), arity: 0, arg_names: vec![], return_type: "Object".into() },
                MethodDesc { name: "REBOOT%".into(), arity: 0, arg_names: vec![], return_type: "Integer".into() },
                MethodDesc { name: "LAST_ERR$".into(), arity: 0, arg_names: vec![], return_type: "String".into() },
            ],
            examples: vec![],
        },
        constants: || Vec::new(),
    });

    add_type("BTC_ALERTS", TypeInfo {
        factory: |_args| {
            Ok(Rc::new(RefCell::new(BtcAlerts {
                eval: AlertEvaluator::new(),
            })))
        },
        descriptor: || ObjectDescriptor {
            type_name: "BTC_ALERTS".into(),
            version: "1.0".into(),
            summary: "Bitcoin Miner Alert Evaluator".into(),
            properties: vec![],
            methods: vec![
                MethodDesc { name: "RULE_TEMP_MAX%".into(), arity: 1, arg_names: vec!["celsius#".into()], return_type: "Integer".into() },
                MethodDesc { name: "RULE_HASHRATE_MIN%".into(), arity: 1, arg_names: vec!["ghs#".into()], return_type: "Integer".into() },
                MethodDesc { name: "RULE_REJECT_RATE_MAX%".into(), arity: 1, arg_names: vec!["pct#".into()], return_type: "Integer".into() },
                MethodDesc { name: "EVALUATE@".into(), arity: 1, arg_names: vec!["snapshot@".into()], return_type: "List".into() },
            ],
            examples: vec![],
        },
        constants: || Vec::new(),
    });

    add_type("BTC_LOG_SQLITE", TypeInfo {
        factory: |args| {
            let path = if args.is_empty() { "btcminer.db".to_string() } else { args[0].to_string() };
            Ok(Rc::new(RefCell::new(BtcLogSqlite {
                logger: SqliteLogger::new(path),
                last_err: None,
            })))
        },
        descriptor: || ObjectDescriptor {
            type_name: "BTC_LOG_SQLITE".into(),
            version: "1.0".into(),
            summary: "Bitcoin Miner SQLite Logger".into(),
            properties: vec![],
            methods: vec![
                MethodDesc { name: "INIT%".into(), arity: 0, arg_names: vec![], return_type: "Integer".into() },
                MethodDesc { name: "WRITE_SNAPSHOT%".into(), arity: 1, arg_names: vec!["snapshot@".into()], return_type: "Integer".into() },
                MethodDesc { name: "LAST_ERR$".into(), arity: 0, arg_names: vec![], return_type: "String".into() },
            ],
            examples: vec![],
        },
        constants: || Vec::new(),
    });
}

fn json_to_basil(jv: serde_json::Value) -> Value {
    match jv {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() { Value::Int(i) }
            else { Value::Num(n.as_f64().unwrap_or(0.0)) }
        }
        serde_json::Value::String(s) => Value::Str(s),
        serde_json::Value::Array(arr) => {
            let vec: Vec<Value> = arr.into_iter().map(json_to_basil).collect();
            Value::List(Rc::new(RefCell::new(vec)))
        }
        serde_json::Value::Object(obj) => {
            let mut map = HashMap::new();
            for (k, v) in obj {
                map.insert(k, json_to_basil(v));
            }
            Value::Dict(Rc::new(RefCell::new(map)))
        }
    }
}

fn basil_to_json(v: &Value) -> serde_json::Value {
    match v {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => serde_json::Value::Number((*i).into()),
        Value::Num(n) => serde_json::Value::Number(serde_json::Number::from_f64(*n).unwrap_or(serde_json::Number::from(0))),
        Value::Str(s) => serde_json::Value::String(s.clone()),
        Value::List(l) => {
            let arr = l.borrow().iter().map(basil_to_json).collect();
            serde_json::Value::Array(arr)
        }
        Value::Dict(d) => {
            let mut obj = serde_json::Map::new();
            for (k, val) in d.borrow().iter() {
                obj.insert(k.clone(), basil_to_json(val));
            }
            serde_json::Value::Object(obj)
        }
        _ => serde_json::Value::Null,
    }
}

struct BtcMinerMgr {
    mgr: MinerManager,
}

impl BasicObject for BtcMinerMgr {
    fn type_name(&self) -> &str { "BTC_MINER_MGR" }
    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("No properties on BTC_MINER_MGR".into())) }
    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("No properties on BTC_MINER_MGR".into())) }
    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "ADD_ASIC%" => {
                let name = args.get(0).map(|v| v.to_string()).unwrap_or_default();
                let kind = args.get(1).map(|v| v.to_string()).unwrap_or_default();
                let host = args.get(2).map(|v| v.to_string()).unwrap_or_default();
                let user = args.get(3).map(|v| v.to_string()).unwrap_or_default();
                let pass = args.get(4).map(|v| v.to_string()).unwrap_or_default();
                
                let profile = if kind == "generic-http-json" {
                     let prof_json = include_str!("../../btcminer-core/profiles/antminer_http_v1.json");
                     Some(serde_json::from_str::<ProfileConfig>(prof_json).map_err(|e| BasilError(e.to_string()))?)
                } else { None };

                match self.mgr.add_asic(name, kind, host, user, pass, profile) {
                    Ok(_) => Ok(Value::Int(1)),
                    Err(_) => Ok(Value::Int(0)),
                }
            }
            "REMOVE_ASIC%" => {
                let name = args.get(0).map(|v| v.to_string()).unwrap_or_default();
                Ok(Value::Int(if self.mgr.remove_asic(&name) { 1 } else { 0 }))
            }
            "SNAPSHOT@" => {
                let snap = self.mgr.take_snapshot();
                let jv = serde_json::to_value(&snap).map_err(|e| BasilError(e.to_string()))?;
                Ok(json_to_basil(jv))
            }
            "PROFILES@" => {
                let list = vec![
                    Value::Str("antminer_http_v1".to_string()),
                    Value::Str("whatsminer_http_v1".to_string()),
                    Value::Str("braiins_os_v1".to_string()),
                ];
                Ok(Value::List(Rc::new(RefCell::new(list))))
            }
            other => Err(BasilError(format!("Unknown method '{}' on BTC_MINER_MGR", other))),
        }
    }
    fn descriptor(&self) -> ObjectDescriptor {
        ObjectDescriptor {
            type_name: "BTC_MINER_MGR".into(),
            version: "1.0".into(),
            summary: "Bitcoin Miner Manager".into(),
            properties: vec![],
            methods: vec![],
            examples: vec![],
        }
    }
}

struct BtcAsic {
    adapter: Box<dyn btcminer_core::adapter::AsicAdapter>,
    last_err: Option<String>,
}

impl BasicObject for BtcAsic {
    fn type_name(&self) -> &str { "BTC_ASIC" }
    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("No properties on BTC_ASIC".into())) }
    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("No properties on BTC_ASIC".into())) }
    fn call(&mut self, method: &str, _args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "INFO@" => {
                match self.adapter.info() {
                    Ok(info) => {
                        let jv = serde_json::to_value(&info).map_err(|e| BasilError(e.to_string()))?;
                        Ok(json_to_basil(jv))
                    }
                    Err(e) => { self.last_err = Some(e.clone()); Err(BasilError(e)) }
                }
            }
            "STATS@" => {
                match self.adapter.stats() {
                    Ok(stats) => {
                        let jv = serde_json::to_value(&stats).map_err(|e| BasilError(e.to_string()))?;
                        Ok(json_to_basil(jv))
                    }
                    Err(e) => { self.last_err = Some(e.clone()); Err(BasilError(e)) }
                }
            }
            "REBOOT%" => {
                match self.adapter.reboot() {
                    Ok(_) => Ok(Value::Int(1)),
                    Err(e) => { self.last_err = Some(e); Ok(Value::Int(0)) }
                }
            }
            "LAST_ERR$" => Ok(Value::Str(self.last_err.clone().unwrap_or_default())),
            other => Err(BasilError(format!("Unknown method '{}' on BTC_ASIC", other))),
        }
    }
    fn descriptor(&self) -> ObjectDescriptor {
        ObjectDescriptor {
            type_name: "BTC_ASIC".into(),
            version: "1.0".into(),
            summary: "Individual Bitcoin ASIC".into(),
            properties: vec![],
            methods: vec![],
            examples: vec![],
        }
    }
}

struct BtcAlerts {
    eval: AlertEvaluator,
}

impl BasicObject for BtcAlerts {
    fn type_name(&self) -> &str { "BTC_ALERTS" }
    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("No properties on BTC_ALERTS".into())) }
    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("No properties on BTC_ALERTS".into())) }
    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "RULE_TEMP_MAX%" => {
                self.eval.rules.temp_max = args.get(0).and_then(|v| match v { Value::Num(n)=>Some(*n), Value::Int(i)=>Some(*i as f64), _=>None });
                Ok(Value::Int(1))
            }
            "RULE_HASHRATE_MIN%" => {
                self.eval.rules.hashrate_min = args.get(0).and_then(|v| match v { Value::Num(n)=>Some(*n), Value::Int(i)=>Some(*i as f64), _=>None });
                Ok(Value::Int(1))
            }
            "RULE_REJECT_RATE_MAX%" => {
                self.eval.rules.reject_rate_max = args.get(0).and_then(|v| match v { Value::Num(n)=>Some(*n), Value::Int(i)=>Some(*i as f64), _=>None });
                Ok(Value::Int(1))
            }
            "EVALUATE@" => {
                let snap_val = args.get(0).ok_or_else(|| BasilError("EVALUATE@: snapshot required".into()))?;
                let jv = basil_to_json(snap_val);
                let snap: MinerSnapshot = serde_json::from_value(jv).map_err(|e| BasilError(format!("Invalid snapshot: {}", e)))?;
                let alerts = self.eval.evaluate(&snap);
                let list = alerts.into_iter().map(Value::Str).collect();
                Ok(Value::List(Rc::new(RefCell::new(list))))
            }
            other => Err(BasilError(format!("Unknown method '{}' on BTC_ALERTS", other))),
        }
    }
    fn descriptor(&self) -> ObjectDescriptor {
        ObjectDescriptor {
            type_name: "BTC_ALERTS".into(),
            version: "1.0".into(),
            summary: "Bitcoin Miner Alert Evaluator".into(),
            properties: vec![],
            methods: vec![],
            examples: vec![],
        }
    }
}

struct BtcLogSqlite {
    logger: SqliteLogger,
    last_err: Option<String>,
}

impl BasicObject for BtcLogSqlite {
    fn type_name(&self) -> &str { "BTC_LOG_SQLITE" }
    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("No properties on BTC_LOG_SQLITE".into())) }
    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("No properties on BTC_LOG_SQLITE".into())) }
    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "INIT%" => {
                match self.logger.init() {
                    Ok(_) => Ok(Value::Int(1)),
                    Err(e) => { self.last_err = Some(e); Ok(Value::Int(0)) }
                }
            }
            "WRITE_SNAPSHOT%" => {
                let snap_val = args.get(0).ok_or_else(|| BasilError("WRITE_SNAPSHOT%: snapshot required".into()))?;
                let jv = basil_to_json(snap_val);
                let snap: MinerSnapshot = serde_json::from_value(jv).map_err(|e| BasilError(format!("Invalid snapshot: {}", e)))?;
                match self.logger.write(&snap) {
                    Ok(_) => Ok(Value::Int(1)),
                    Err(e) => { self.last_err = Some(e); Ok(Value::Int(0)) }
                }
            }
            "LAST_ERR$" => Ok(Value::Str(self.last_err.clone().unwrap_or_default())),
            other => Err(BasilError(format!("Unknown method '{}' on BTC_LOG_SQLITE", other))),
        }
    }
    fn descriptor(&self) -> ObjectDescriptor {
        ObjectDescriptor {
            type_name: "BTC_LOG_SQLITE".into(),
            version: "1.0".into(),
            summary: "Bitcoin Miner SQLite Logger".into(),
            properties: vec![],
            methods: vec![],
            examples: vec![],
        }
    }
}
