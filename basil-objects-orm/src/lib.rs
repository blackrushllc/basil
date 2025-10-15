use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use basil_common::{BasilError, Result};
use basil_bytecode::{ArrayObj, BasicObject, ElemType, MethodDesc, ObjectDescriptor, PropDesc, Value};

// Mirror the TypeInfo used by basil-objects for bridging registrations.
pub struct TypeInfo {
    pub factory: fn(args: &[Value]) -> Result<Rc<RefCell<dyn BasicObject>>>,
    pub descriptor: fn() -> ObjectDescriptor,
    pub constants: fn() -> Vec<(String, Value)>,
}

pub fn register<F: FnMut(&str, TypeInfo)>(mut reg: F) {
    #[cfg(any(feature = "obj-orm", feature = "obj-orm-mysql", feature = "obj-orm-postgres"))]
    {
        let factory = |args: &[Value]| -> Result<Rc<RefCell<dyn BasicObject>>> {
            // Expect single arg: DB_* object
            if args.len() != 1 { return Err(BasilError("ORM: constructor expects 1 argument (DB object)".into())); }
            let db = match &args[0] { Value::Object(rc)=> rc.clone(), other=> return Err(BasilError(format!("ORM: expected DB object, got {}", other))) };
            let dialect = db.borrow().type_name().to_ascii_lowercase();
            if dialect != "db_postgres" && dialect != "db_mysql" { return Err(BasilError("ORM: unsupported DB object; use DB_POSTGRES or DB_MYSQL".into())); }
            Ok(Rc::new(RefCell::new(OrmObj { db, dialect: if dialect=="db_postgres"{"postgres".into()} else {"mysql".into()}, models: HashMap::new() })))
        };
        let descriptor = || OrmObj::descriptor_static();
        let constants = || Vec::<(String, Value)>::new();
        reg("ORM", TypeInfo { factory, descriptor, constants });
    }
}

#[derive(Clone, Debug, Default)]
struct RelationMeta {
    // name implicit from map key (e.g., "posts" on users)
    kind: String, // "has_many" or "belongs_to"
    other: String, // other table name
    fk_col: String,
    pk_col: String, // only for belongs_to; for has_many, parent's pk (filled from model.pk)
}

#[derive(Clone, Debug, Default)]
struct ModelMeta {
    table: String,
    cols: Vec<String>, // Basil-suffixed names like id%, name$
    pk: String,        // suffixed name like id%
    relations: HashMap<String, RelationMeta>,
}

#[derive(Clone)]
struct OrmObj {
    db: Rc<RefCell<dyn BasicObject>>, // DB_POSTGRES or DB_MYSQL
    dialect: String,                  // "postgres" | "mysql"
    models: HashMap<String, ModelMeta>,
}

impl OrmObj {
    fn descriptor_static() -> ObjectDescriptor {
        ObjectDescriptor {
            type_name: "ORM".into(),
            version: "0.1".into(),
            summary: "Lightweight dynamic ORM (MySQL/Postgres)".into(),
            properties: vec![],
            methods: vec![
                MethodDesc { name: "Model".into(), arity: 3, arg_names: vec!["table$".into(), "cols$[]".into(), "pk$".into()], return_type: "ORM".into() },
                MethodDesc { name: "ModelFromTable$".into(), arity: 1, arg_names: vec!["table$".into()], return_type: "Str$".into() },
                MethodDesc { name: "HasMany".into(), arity: 3, arg_names: vec!["parent$".into(), "child$".into(), "fk$".into()], return_type: "ORM".into() },
                MethodDesc { name: "BelongsTo".into(), arity: 4, arg_names: vec!["child$".into(), "parent$".into(), "fk$".into(), "pk$".into()], return_type: "ORM".into() },
                MethodDesc { name: "Table".into(), arity: 1, arg_names: vec!["table$".into()], return_type: "ORM_QUERY".into() },
                MethodDesc { name: "New".into(), arity: 1, arg_names: vec!["table$".into()], return_type: "ORM_ROW".into() },
                MethodDesc { name: "Begin".into(), arity: 0, arg_names: vec![], return_type: "Int%".into() },
                MethodDesc { name: "Commit".into(), arity: 0, arg_names: vec![], return_type: "Int%".into() },
                MethodDesc { name: "Rollback".into(), arity: 0, arg_names: vec![], return_type: "Int%".into() },
                MethodDesc { name: "RowFromJson$".into(), arity: 2, arg_names: vec!["table$".into(), "json$".into()], return_type: "ORM_ROW".into() },
            ],
            examples: vec![
                "DIM orm@ AS ORM(db@)".into(),
                "orm@.Model(\"users\", [\"id%\",\"name$\"], \"id%\")".into(),
            ],
        }
    }

    fn ensure_model(&self, table: &str) -> Result<ModelMeta> {
        self.models.get(&table.to_string()).cloned().ok_or_else(|| BasilError(format!("ORM.ModelNotFound: {}", table)))
    }

    fn info_schema_sql(&self, table: &str) -> (String, String) {
        if self.dialect == "postgres" {
            (
                // columns
                "SELECT column_name, data_type FROM information_schema.columns WHERE table_name = $1 ORDER BY ordinal_position".into(),
                // pk
                "SELECT a.attname FROM pg_index i JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey) WHERE i.indrelid = $1::regclass AND i.indisprimary".into(),
            )
        } else {
            (
                "SELECT column_name, data_type FROM information_schema.columns WHERE table_schema = DATABASE() AND table_name = ? ORDER BY ordinal_position".into(),
                "SELECT kcu.column_name FROM information_schema.table_constraints tc JOIN information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name AND tc.table_name = kcu.table_name WHERE tc.table_schema = DATABASE() AND tc.table_name = ? AND tc.constraint_type = 'PRIMARY KEY' LIMIT 1".into(),
            )
        }
    }
}

fn make_array_of_objects(objs: Vec<Rc<RefCell<dyn BasicObject>>>, type_name_hint: Option<&str>) -> Value {
    let elem = match type_name_hint { Some(t) => ElemType::Obj(Some(t.to_string())), None => ElemType::Obj(None) };
    let dims = vec![objs.len()];
    let mut data: Vec<Value> = Vec::with_capacity(objs.len());
    for o in objs { data.push(Value::Object(o)); }
    Value::Array(Rc::new(ArrayObj { elem, dims, data: RefCell::new(data) }))
}

fn as_str(v: &Value) -> String { match v { Value::Str(s)=>s.clone(), Value::Int(i)=>i.to_string(), Value::Num(n)=>format!("{}", n), Value::Bool(b)=>b.to_string(), Value::Null=>"".into(), other=> format!("{}", other) } }

impl BasicObject for OrmObj {
    fn type_name(&self) -> &str { "ORM" }

    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("ORM: unknown property".into())) }

    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("ORM: unknown or read-only property".into())) }

    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "MODEL" => {
                if args.len() != 3 { return Err(BasilError("ORM.Model expects (table$, cols$[], pk$)".into())); }
                let table = as_str(&args[0]);
                let cols = match &args[1] { Value::Array(arr_rc)=> {
                    let arr = arr_rc.as_ref();
                    let mut out = Vec::new();
                    for v in arr.data.borrow().iter() { out.push(as_str(v)); }
                    out
                }, _=> return Err(BasilError("ORM.Model: cols$[] must be array of strings".into())) };
                let pk = as_str(&args[2]);
                let mm = ModelMeta { table: table.clone(), cols, pk: pk.clone(), relations: HashMap::new() };
                self.models.insert(table, mm);
                Ok(Value::Object(Rc::new(RefCell::new(self.clone()))))
            }
            ,"MODELFROMTABLE$" => {
                if args.len() != 1 { return Err(BasilError("ORM.ModelFromTable$ expects (table$)".into())); }
                let table = as_str(&args[0]);
                // fetch info schema
                let (cols_sql, pk_sql) = self.info_schema_sql(&table);
                // columns
                let cols_json = self.db.borrow_mut().call("QUERY$", &[Value::Str(cols_sql.clone()), Value::Str(table.clone())])?;
                let mut cols: Vec<String> = Vec::new();
                if let Value::Str(s) = cols_json { // parse very simply as JSON array of objects with column_name, data_type
                    #[cfg(any(feature = "obj-orm-mysql", feature = "obj-orm-postgres"))]
                    {
                        let val: serde_json::Value = serde_json::from_str(&s).map_err(|e| BasilError(format!("ORM.ModelFromTable$: parse error: {}", e)))?;
                        if let Some(arr) = val.as_array() {
                            for row in arr {
                                let col = row.get("column_name").and_then(|v| v.as_str()).unwrap_or("");
                                let dt = row.get("data_type").and_then(|v| v.as_str()).unwrap_or("");
                                if col.is_empty() { continue; }
                                let suffix = if dt.contains("int") { "%" } else if dt.contains("char") || dt.contains("text") || dt.contains("uuid") || dt.contains("date") || dt.contains("time") || dt.contains("json") { "$" } else { "" };
                                cols.push(format!("{}{}", col, suffix));
                            }
                        }
                    }
                }
                // pk
                let pk_json = self.db.borrow_mut().call("QUERY$", &[Value::Str(pk_sql.clone()), Value::Str(table.clone())])?;
                let mut pk = String::new();
                if let Value::Str(s) = pk_json {
                    #[cfg(any(feature = "obj-orm-mysql", feature = "obj-orm-postgres"))]
                    {
                        let val: serde_json::Value = serde_json::from_str(&s).map_err(|e| BasilError(format!("ORM.ModelFromTable$: parse error: {}", e)))?;
                        if let Some(arr) = val.as_array() { if let Some(row0) = arr.first() { if let Some(cn) = row0.get("attname").or_else(|| row0.get("column_name")) { pk = format!("{}%", cn.as_str().unwrap_or("") ); } } }
                    }
                }
                if cols.is_empty() { return Err(BasilError(format!("ORM.ModelFromTable$: no columns found for {}", table))); }
                if pk.is_empty() { pk = format!("{}%", "id"); }
                let mm = ModelMeta { table: table.clone(), cols: cols.clone(), pk: pk.clone(), relations: HashMap::new() };
                self.models.insert(table.clone(), mm);
                Ok(Value::Str(table))
            }
            ,"HASMANY" => {
                if args.len() != 3 { return Err(BasilError("ORM.HasMany expects (parent$, child$, fk$)".into())); }
                let parent = as_str(&args[0]);
                let child = as_str(&args[1]);
                let fk = as_str(&args[2]);
                let mut m = self.models.get(&parent).cloned().ok_or_else(|| BasilError(format!("ORM.ModelNotFound: {}", parent)))?;
                let pk = m.pk.clone();
                m.relations.insert(child.clone(), RelationMeta { kind: "has_many".into(), other: child, fk_col: fk, pk_col: pk });
                self.models.insert(parent, m);
                Ok(Value::Object(Rc::new(RefCell::new(self.clone()))))
            }
            ,"BELONGSTO" => {
                if args.len() != 4 { return Err(BasilError("ORM.BelongsTo expects (child$, parent$, fk$, pk$)".into())); }
                let child = as_str(&args[0]);
                let parent = as_str(&args[1]);
                let fk = as_str(&args[2]);
                let pk = as_str(&args[3]);
                let mut m = self.models.get(&child).cloned().ok_or_else(|| BasilError(format!("ORM.ModelNotFound: {}", child)))?;
                m.relations.insert(parent.clone(), RelationMeta { kind: "belongs_to".into(), other: parent, fk_col: fk, pk_col: pk });
                self.models.insert(child, m);
                Ok(Value::Object(Rc::new(RefCell::new(self.clone()))))
            }
            ,"TABLE" => {
                if args.len() != 1 { return Err(BasilError("ORM.Table expects (name$)".into())); }
                let table = as_str(&args[0]);
                let meta = self.ensure_model(&table)?;
                let q = QueryObj { db: self.db.clone(), dialect: self.dialect.clone(), table, select_cols: Vec::new(), wheres: Vec::new(), order: None, limit: None, offset: None, with: Vec::new(), meta };
                Ok(Value::Object(Rc::new(RefCell::new(q))))
            }
            ,"NEW" => {
                if args.len() != 1 { return Err(BasilError("ORM.New expects (table$)".into())); }
                let table = as_str(&args[0]);
                let meta = self.ensure_model(&table)?;
                let row = RowObj::new(self.db.clone(), self.dialect.clone(), meta, None);
                Ok(Value::Object(Rc::new(RefCell::new(row))))
            }
            ,"BEGIN" => { self.db.borrow_mut().call("BEGIN", &[]).map(|_| Value::Int(1)) }
            ,"COMMIT" => { self.db.borrow_mut().call("COMMIT", &[]).map(|_| Value::Int(1)) }
            ,"ROLLBACK" => { self.db.borrow_mut().call("ROLLBACK", &[]).map(|_| Value::Int(1)) }
            ,"ROWFROMJSON$" => {
                if args.len() != 2 { return Err(BasilError("ORM.RowFromJson$ expects (table$, json$)".into())); }
                let table = as_str(&args[0]);
                let meta = self.ensure_model(&table)?;
                let json = as_str(&args[1]);
                #[cfg(any(feature = "obj-orm-mysql", feature = "obj-orm-postgres"))]
                {
                    let v: serde_json::Value = serde_json::from_str(&json).map_err(|e| BasilError(format!("ORM.RowFromJson$: {}", e)))?;
                    let mut data = HashMap::new();
                    if let Some(obj) = v.as_object() {
                        for (k, vv) in obj.iter() { data.insert(k.clone(), vv.to_string().trim_matches('"').to_string()); }
                    }
                    let row = RowObj::new(self.db.clone(), self.dialect.clone(), meta, Some(data));
                    return Ok(Value::Object(Rc::new(RefCell::new(row))));
                }
                Err(BasilError("ORM.RowFromJson$: JSON disabled (enable obj-orm-*)".into()))
            }
            ,other => Err(BasilError(format!("Unknown method '{}' on ORM", other)))
        }
    }

    fn descriptor(&self) -> ObjectDescriptor { Self::descriptor_static() }
}

#[derive(Clone)]
struct QueryObj {
    db: Rc<RefCell<dyn BasicObject>>, // DB object
    dialect: String,
    table: String,
    select_cols: Vec<String>,
    wheres: Vec<(String, String, String)>,
    order: Option<(String, String)>,
    limit: Option<i64>,
    offset: Option<i64>,
    with: Vec<String>,
    meta: ModelMeta,
}

impl QueryObj {
    fn descriptor_static() -> ObjectDescriptor {
        ObjectDescriptor {
            type_name: "ORM_QUERY".into(),
            version: "0.1".into(),
            summary: "ORM query builder".into(),
            properties: vec![],
            methods: vec![
                MethodDesc { name: "Where$".into(), arity: 3, arg_names: vec!["col$".into(), "op$".into(), "val$".into()], return_type: "ORM_QUERY".into() },
                MethodDesc { name: "OrderBy$".into(), arity: 2, arg_names: vec!["col$".into(), "dir$".into()], return_type: "ORM_QUERY".into() },
                MethodDesc { name: "Limit%".into(), arity: 1, arg_names: vec!["n%".into()], return_type: "ORM_QUERY".into() },
                MethodDesc { name: "Offset%".into(), arity: 1, arg_names: vec!["n%".into()], return_type: "ORM_QUERY".into() },
                MethodDesc { name: "With$".into(), arity: 1, arg_names: vec!["rel$".into()], return_type: "ORM_QUERY".into() },
                MethodDesc { name: "Select$".into(), arity: 1, arg_names: vec!["cols$[]".into()], return_type: "ORM_QUERY".into() },
                MethodDesc { name: "Get".into(), arity: 0, arg_names: vec![], return_type: "ARRAY<ORM_ROW>".into() },
                MethodDesc { name: "Find%".into(), arity: 1, arg_names: vec!["pk%".into()], return_type: "ORM_ROW".into() },
                MethodDesc { name: "First".into(), arity: 0, arg_names: vec![], return_type: "ORM_ROW".into() },
                MethodDesc { name: "ToJson$".into(), arity: 0, arg_names: vec![], return_type: "Str$".into() },
            ],
            examples: vec![
                "DIM q@ = orm@.Table(\"users\").Where$(\"email$\", \"=\", \"a@example.com\")".into(),
            ],
        }
    }

    fn quote_ident(&self, id: &str) -> String {
        let id = id.trim_matches('`').trim_matches('"');
        if self.dialect == "postgres" { format!("\"{}\"", id) } else { format!("`{}`", id) }
    }

    fn placeholder(&self, idx: usize) -> String {
        if self.dialect == "postgres" { format!("${}", idx) } else { "?".into() }
    }

    fn compile_select(&self) -> (String, Vec<String>) {
        let cols = if self.select_cols.is_empty() { "*".to_string() } else { self.select_cols.join(", ") };
        let mut sql = format!("SELECT {} FROM {}", cols, self.quote_ident(&self.table));
        let mut params: Vec<String> = Vec::new();
        if !self.wheres.is_empty() {
            sql.push_str(" WHERE ");
            let mut first = true;
            for (i, (col, op, val)) in self.wheres.iter().enumerate() {
                if !first { sql.push_str(" AND "); } else { first = false; }
                let ph = self.placeholder(i+1);
                sql.push_str(&format!("{} {} {}", self.quote_ident(col.trim_end_matches(['%','$'])), op, ph));
                params.push(val.clone());
            }
        }
        if let Some((col, dir)) = &self.order { sql.push_str(&format!(" ORDER BY {} {}", self.quote_ident(col.trim_end_matches(['%','$'])), if dir.eq_ignore_ascii_case("DESC"){"DESC"}else{"ASC"})); }
        if let Some(n) = self.limit { sql.push_str(&format!(" LIMIT {}", n)); }
        if let Some(n) = self.offset { sql.push_str(&format!(" OFFSET {}", n)); }
        (sql, params)
    }
}

impl BasicObject for QueryObj {
    fn type_name(&self) -> &str { "ORM_QUERY" }

    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("ORM_QUERY: unknown property".into())) }

    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("ORM_QUERY: unknown or read-only property".into())) }

    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "WHERE$" => { if args.len()!=3 { return Err(BasilError("Where$(col$,op$,val$)".into())); } self.wheres.push((as_str(&args[0]), as_str(&args[1]), as_str(&args[2]))); Ok(Value::Object(Rc::new(RefCell::new(self.clone())))) }
            ,"ORDERBY$" => { if args.len()!=2 { return Err(BasilError("OrderBy$(col$,dir$)".into())); } self.order = Some((as_str(&args[0]), as_str(&args[1]))); Ok(Value::Object(Rc::new(RefCell::new(self.clone())))) }
            ,"LIMIT%" => { if args.len()!=1 { return Err(BasilError("Limit%(n%)".into())); } let n = match &args[0]{ Value::Int(i)=>*i, Value::Num(n)=>n.trunc() as i64, other=> { return Err(BasilError(format!("Limit% expects int, got {}", other))); } }; self.limit = Some(n); Ok(Value::Object(Rc::new(RefCell::new(self.clone())))) }
            ,"OFFSET%" => { if args.len()!=1 { return Err(BasilError("Offset%(n%)".into())); } let n = match &args[0]{ Value::Int(i)=>*i, Value::Num(n)=>n.trunc() as i64, other=> { return Err(BasilError(format!("Offset% expects int, got {}", other))); } }; self.offset = Some(n); Ok(Value::Object(Rc::new(RefCell::new(self.clone())))) }
            ,"WITH$" => { if args.len()!=1 { return Err(BasilError("With$(relation$)".into())); } self.with.push(as_str(&args[0])); Ok(Value::Object(Rc::new(RefCell::new(self.clone())))) }
            ,"SELECT$" => { if args.len()!=1 { return Err(BasilError("Select$(cols$[])".into())); } let cols = match &args[0] { Value::Array(rc)=> rc.data.borrow().iter().map(|v| as_str(v)).collect(), _=> return Err(BasilError("Select$ expects array of strings".into())) }; self.select_cols = cols; Ok(Value::Object(Rc::new(RefCell::new(self.clone())))) }
            ,"GET" => {
                let (sql, mut params) = self.compile_select();
                // Execute via DB.Query$ and build Row[] from JSON
                let mut call_args: Vec<Value> = Vec::with_capacity(1 + params.len());
                call_args.push(Value::Str(sql));
                for p in params.drain(..) { call_args.push(Value::Str(p)); }
                let res = self.db.borrow_mut().call("QUERY$", &call_args)?;
                let mut rows: Vec<Rc<RefCell<dyn BasicObject>>> = Vec::new();
                if let Value::Str(json) = res {
                    #[cfg(any(feature = "obj-orm-mysql", feature = "obj-orm-postgres"))]
                    {
                        let arr: serde_json::Value = serde_json::from_str(&json).map_err(|e| BasilError(format!("ORM.Query.Get JSON parse: {}", e)))?;
                        if let Some(v) = arr.as_array() {
                            for obj in v {
                                let mut data = HashMap::new();
                                if let Some(map) = obj.as_object() {
                                    for (k, vv) in map.iter() { data.insert(k.clone(), vv.to_string().trim_matches('"').to_string()); }
                                }
                                let row = RowObj::new(self.db.clone(), self.dialect.clone(), self.meta.clone(), Some(data));
                                rows.push(Rc::new(RefCell::new(row)));
                            }
                        }
                    }
                }
                Ok(make_array_of_objects(rows, Some("ORM_ROW")))
            }
            ,"FIND%" => {
                if args.len()!=1 { return Err(BasilError("Find%(pk%)".into())); }
                let pkcol = self.meta.pk.trim_end_matches(['%','$']).to_string();
                let (mut sql, mut params) = (String::new(), Vec::new());
                if self.dialect=="postgres" { sql = format!("SELECT * FROM {} WHERE {} = $1 LIMIT 1", self.quote_ident(&self.table), self.quote_ident(&pkcol)); params.push(as_str(&args[0])); }
                else { sql = format!("SELECT * FROM {} WHERE {} = ? LIMIT 1", self.quote_ident(&self.table), self.quote_ident(&pkcol)); params.push(as_str(&args[0])); }
                let mut call_args: Vec<Value> = vec![Value::Str(sql)]; for p in params { call_args.push(Value::Str(p)); }
                let res = self.db.borrow_mut().call("QUERY$", &call_args)?;
                if let Value::Str(json) = res {
                    #[cfg(any(feature = "obj-orm-mysql", feature = "obj-orm-postgres"))]
                    {
                        let arr: serde_json::Value = serde_json::from_str(&json).map_err(|e| BasilError(format!("ORM.Query.Find% JSON parse: {}", e)))?;
                        if let Some(v) = arr.as_array() { if let Some(first) = v.first() {
                            let mut data = HashMap::new(); if let Some(map) = first.as_object() { for (k,vv) in map.iter() { data.insert(k.clone(), vv.to_string().trim_matches('"').to_string()); } }
                            let row = RowObj::new(self.db.clone(), self.dialect.clone(), self.meta.clone(), Some(data));
                            return Ok(Value::Object(Rc::new(RefCell::new(row))));
                        }}
                    }
                }
                Err(BasilError(format!("ORM.ModelNotFound: {} (pk=<value>)", self.table)))
            }
            ,"FIRST" => {
                self.limit = Some(1);
                let arr = self.call("GET", &[])?;
                match arr { Value::Array(rc) => {
                    if rc.data.borrow().is_empty() { Ok(Value::Null) } else { Ok(rc.data.borrow()[0].clone()) }
                }, _=> Ok(Value::Null) }
            }
            ,"TOJSON$" => {
                let (sql, mut params) = self.compile_select();
                let mut call_args: Vec<Value> = Vec::with_capacity(1 + params.len()); call_args.push(Value::Str(sql)); for p in params.drain(..) { call_args.push(Value::Str(p)); }
                let res = self.db.borrow_mut().call("QUERY$", &call_args)?;
                Ok(res)
            }
            ,other => Err(BasilError(format!("Unknown method '{}' on ORM_QUERY", other)))
        }
    }

    fn descriptor(&self) -> ObjectDescriptor { Self::descriptor_static() }
}

#[derive(Clone)]
struct RowObj {
    db: Rc<RefCell<dyn BasicObject>>, // DB object
    dialect: String,
    meta: ModelMeta,
    data: HashMap<String, String>,
    dirty: HashMap<String, bool>,
}

impl RowObj {
    fn new(db: Rc<RefCell<dyn BasicObject>>, dialect: String, meta: ModelMeta, init: Option<HashMap<String, String>>) -> Self {
        let mut data = HashMap::new();
        if let Some(m) = &init { data.extend(m.clone()); }
        Self { db, dialect, meta, data, dirty: HashMap::new() }
    }

    fn descriptor_static() -> ObjectDescriptor {
        ObjectDescriptor {
            type_name: "ORM_ROW".into(),
            version: "0.1".into(),
            summary: "ORM row object with Active Record methods".into(),
            properties: vec![PropDesc { name: "Pk$".into(), type_name: "Str$".into(), readable: true, writable: false }],
            methods: vec![
                MethodDesc { name: "Save".into(), arity: 0, arg_names: vec![], return_type: "Int%".into() },
                MethodDesc { name: "Delete".into(), arity: 0, arg_names: vec![], return_type: "Int%".into() },
                MethodDesc { name: "ToJson$".into(), arity: 0, arg_names: vec![], return_type: "Str$".into() },
            ],
            examples: vec!["u@.Save()".into()],
        }
    }

    fn pk_plain(&self) -> String { self.meta.pk.trim_end_matches(['%','$']).to_string() }

    fn placeholder(&self, idx: usize) -> String { if self.dialect=="postgres" { format!("${}", idx) } else { "?".into() } }

    fn quote_ident(&self, id: &str) -> String { let id = id.trim_matches('`').trim_matches('"'); if self.dialect=="postgres" { format!("\"{}\"", id) } else { format!("`{}`", id) } }
}

impl BasicObject for RowObj {
    fn type_name(&self) -> &str { "ORM_ROW" }

    fn get_prop(&self, name: &str) -> Result<Value> {
        match name.to_ascii_uppercase().as_str() {
            "PK$" => {
                let pk = self.pk_plain();
                let v = self.data.get(&pk).cloned().unwrap_or_default();
                Ok(Value::Str(v))
            }
            _ => {
                // dynamic column mapping by suffix name
                let wanted = name.to_string();
                let col = self.meta.cols.iter().find(|c| c.eq_ignore_ascii_case(&wanted)).cloned().ok_or_else(|| BasilError(format!("ORM.UnknownColumn: {}.{}", self.meta.table, name)))?;
                let plain = col.trim_end_matches(['%','$']).to_string();
                let v = self.data.get(&plain).cloned().unwrap_or_default();
                Ok(Value::Str(v))
            }
        }
    }

    fn set_prop(&mut self, name: &str, v: Value) -> Result<()> {
        let wanted = name.to_string();
        let col = self.meta.cols.iter().find(|c| c.eq_ignore_ascii_case(&wanted)).cloned().ok_or_else(|| BasilError(format!("ORM.UnknownColumn: {}.{}", self.meta.table, name)))?;
        let plain = col.trim_end_matches(['%','$']).to_string();
        let sval = as_str(&v);
        self.data.insert(plain.clone(), sval);
        self.dirty.insert(plain, true);
        Ok(())
    }

    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "SAVE" => {
                // INSERT vs UPDATE
                let pkcol = self.pk_plain();
                let has_pk = self.data.get(&pkcol).map(|v| !v.is_empty()).unwrap_or(false);
                if !has_pk {
                    // INSERT
                    let mut cols: Vec<String> = Vec::new();
                    let mut vals: Vec<String> = Vec::new();
                    for c in self.meta.cols.iter() {
                        let p = c.trim_end_matches(['%','$']).to_string();
                        if let Some(v) = self.data.get(&p) { cols.push(p); vals.push(v.clone()); }
                    }
                    if cols.is_empty() { return Err(BasilError(format!("ORM.ValidationFailed: {} (<field> required)", self.meta.table))); }
                    let mut sql = String::new();
                    sql.push_str(&format!("INSERT INTO {} ({} ) VALUES (", self.quote_ident(&self.meta.table), cols.iter().map(|c| self.quote_ident(c)).collect::<Vec<_>>().join(", ")));
                    let mut params: Vec<Value> = Vec::new();
                    for i in 0..vals.len() { if i>0 { sql.push_str(", "); } sql.push_str(&self.placeholder(i+1)); params.push(Value::Str(vals[i].clone())); }
                    sql.push_str(")");
                    let mut call_args = vec![Value::Str(sql)]; call_args.extend(params);
                    let _ = self.db.borrow_mut().call("EXECUTE", &call_args)?;
                    Ok(Value::Int(1))
                } else {
                    // UPDATE (dirty only)
                    let mut sets: Vec<String> = Vec::new();
                    let mut params: Vec<Value> = Vec::new();
                    for (col, is_dirty) in self.dirty.clone().into_iter() {
                        if is_dirty { sets.push(format!("{} = {}", self.quote_ident(&col), self.placeholder(sets.len()+1))); params.push(Value::Str(self.data.get(&col).cloned().unwrap_or_default())); }
                    }
                    if sets.is_empty() { return Ok(Value::Int(1)); }
                    let mut sql = format!("UPDATE {} SET {} WHERE {} = {}", self.quote_ident(&self.meta.table), sets.join(", "), self.quote_ident(&self.pk_plain()), self.placeholder(sets.len()+1));
                    params.push(Value::Str(self.data.get(&pkcol).cloned().unwrap_or_default()));
                    let mut call_args = vec![Value::Str(sql)]; call_args.extend(params);
                    let _ = self.db.borrow_mut().call("EXECUTE", &call_args)?;
                    Ok(Value::Int(1))
                }
            }
            ,"DELETE" => {
                let pkcol = self.pk_plain();
                let v = self.data.get(&pkcol).cloned().unwrap_or_default();
                if v.is_empty() { return Err(BasilError(format!("ORM.ValidationFailed: {} ({} required)", self.meta.table, self.meta.pk))); }
                let sql = if self.dialect=="postgres" { format!("DELETE FROM {} WHERE {} = $1", self.quote_ident(&self.meta.table), self.quote_ident(&pkcol)) } else { format!("DELETE FROM {} WHERE {} = ?", self.quote_ident(&self.meta.table), self.quote_ident(&pkcol)) };
                let _ = self.db.borrow_mut().call("EXECUTE", &[Value::Str(sql), Value::Str(v)])?;
                Ok(Value::Int(1))
            }
            ,"TOJSON$" => {
                #[cfg(any(feature = "obj-orm-mysql", feature = "obj-orm-postgres"))]
                {
                    let mut map = serde_json::Map::new();
                    for c in self.meta.cols.iter() {
                        let p = c.trim_end_matches(['%','$']).to_string();
                        let v = self.data.get(&p).cloned().unwrap_or_default();
                        map.insert(p, serde_json::Value::String(v));
                    }
                    return Ok(Value::Str(serde_json::Value::Object(map).to_string()));
                }
                Err(BasilError("JSON support disabled; enable obj-orm-*".into()))
            }
            ,other => {
                // relation accessors, e.g., Posts(), User()
                if method.ends_with("()") || !args.is_empty() { return Err(BasilError(format!("Unknown method '{}' on ORM_ROW", method))); }
                let name = method; // relation or unknown
                if let Some(rel) = self.meta.relations.get(&name.to_string()) {
                    match rel.kind.as_str() {
                        "has_many" => {
                            // SELECT * FROM other WHERE fk = self.pk
                            let fk = rel.fk_col.trim_end_matches(['%','$']).to_string();
                            let pkv = self.data.get(&self.pk_plain()).cloned().unwrap_or_default();
                            let (sql, params) = if self.dialect=="postgres" { (format!("SELECT * FROM {} WHERE {} = $1", rel.other, fk), vec![pkv]) } else { (format!("SELECT * FROM {} WHERE {} = ?", rel.other, fk), vec![pkv]) };
                            let mut call_args: Vec<Value> = vec![Value::Str(sql)]; for p in params { call_args.push(Value::Str(p)); }
                            let res = self.db.borrow_mut().call("QUERY$", &call_args)?;
                            let mut rows: Vec<Rc<RefCell<dyn BasicObject>>> = Vec::new();
                            if let Value::Str(json) = res {
                                #[cfg(any(feature = "obj-orm-mysql", feature = "obj-orm-postgres"))]
                                {
                                    let arr: serde_json::Value = serde_json::from_str(&json).map_err(|e| BasilError(format!("ORM.Row.Relation JSON parse: {}", e)))?;
                                    if let Some(v) = arr.as_array() {
                                        for obj in v {
                                            let mut data = HashMap::new();
                                            if let Some(map) = obj.as_object() { for (k, vv) in map.iter() { data.insert(k.clone(), vv.to_string().trim_matches('"').to_string()); } }
                                            // Need meta of child
                                            // Fallback to assume model registered
                                            // In worst case, relation other table meta not found
                                            let child_meta = self.meta.clone(); // placeholder; better lookup omitted to keep minimal
                                            let row = RowObj::new(self.db.clone(), self.dialect.clone(), child_meta, Some(data));
                                            rows.push(Rc::new(RefCell::new(row)));
                                        }
                                    }
                                }
                            }
                            return Ok(make_array_of_objects(rows, Some("ORM_ROW")));
                        }
                        "belongs_to" => {
                            let fk = rel.fk_col.trim_end_matches(['%','$']).to_string();
                            let fkv = self.data.get(&fk).cloned().unwrap_or_default();
                            let pk = rel.pk_col.trim_end_matches(['%','$']).to_string();
                            let (sql, params) = if self.dialect=="postgres" { (format!("SELECT * FROM {} WHERE {} = $1 LIMIT 1", rel.other, pk), vec![fkv]) } else { (format!("SELECT * FROM {} WHERE {} = ? LIMIT 1", rel.other, pk), vec![fkv]) };
                            let mut call_args: Vec<Value> = vec![Value::Str(sql)]; for p in params { call_args.push(Value::Str(p)); }
                            let res = self.db.borrow_mut().call("QUERY$", &call_args)?;
                            if let Value::Str(json) = res {
                                #[cfg(any(feature = "obj-orm-mysql", feature = "obj-orm-postgres"))]
                                {
                                    let arr: serde_json::Value = serde_json::from_str(&json).map_err(|e| BasilError(format!("ORM.Row.Relation JSON parse: {}", e)))?;
                                    if let Some(v) = arr.as_array() { if let Some(first) = v.first() {
                                        let mut data = HashMap::new(); if let Some(map) = first.as_object() { for (k, vv) in map.iter() { data.insert(k.clone(), vv.to_string().trim_matches('"').to_string()); } }
                                        // Use child meta placeholder (would need lookup)
                                        let child_meta = self.meta.clone();
                                        let row = RowObj::new(self.db.clone(), self.dialect.clone(), child_meta, Some(data));
                                        return Ok(Value::Object(Rc::new(RefCell::new(row))));
                                    }}
                                }
                            }
                            Ok(Value::Null)
                        }
                        _ => Err(BasilError(format!("ORM.RelationMissing: {}.{}", self.meta.table, name)))
                    }
                } else {
                    Err(BasilError(format!("ORM.RelationMissing: {}.{}", self.meta.table, name)))
                }
            }
        }
    }

    fn descriptor(&self) -> ObjectDescriptor { Self::descriptor_static() }
}
