use basil_common::{Result, BasilError};
use basil_bytecode::{Value, ObjectDescriptor, PropDesc, MethodDesc, BasicObject};
use std::rc::Rc;
use std::cell::RefCell;

pub fn register(reg: &mut crate::Registry) {
    reg.register("BMX_RIDER", crate::TypeInfo {
        factory: |args| {
            // Expected: (name$, age%, skill$, wins%, losses%)
            if args.len() != 5 { return Err(BasilError(format!("BMX_RIDER expects 5 args, got {}", args.len()))); }
            let name = match &args[0] { Value::Str(s) => s.clone(), other => format!("{}", other) };
            let age = match &args[1] { Value::Int(i) => *i as i64, Value::Num(n) => n.trunc() as i64, _ => 0 } as i32;
            let skill = match &args[2] { Value::Str(s) => s.clone(), other => format!("{}", other) };
            let wins = match &args[3] { Value::Int(i) => *i, Value::Num(n) => n.trunc() as i64, _ => 0 };
            let losses = match &args[4] { Value::Int(i) => *i, Value::Num(n) => n.trunc() as i64, _ => 0 };
            Ok(Rc::new(RefCell::new(BmxRider { name, age, skill_level: skill, wins, losses })))
        },
        descriptor: descriptor_static,
        constants: || Vec::new(),
    });
}

fn descriptor_static() -> ObjectDescriptor {
    ObjectDescriptor {
        type_name: "BMX_RIDER".to_string(),
        version: "1.0".to_string(),
        summary: "Represents an individual BMX rider".to_string(),
        properties: vec![
            PropDesc { name: "Name$".to_string(), type_name: "String".to_string(), readable: true, writable: true },
            PropDesc { name: "Age%".to_string(), type_name: "Integer".to_string(), readable: true, writable: true },
            PropDesc { name: "SkillLevel$".to_string(), type_name: "String".to_string(), readable: true, writable: true },
            PropDesc { name: "Wins%".to_string(), type_name: "Integer".to_string(), readable: true, writable: true },
            PropDesc { name: "Losses%".to_string(), type_name: "Integer".to_string(), readable: true, writable: true },
        ],
        methods: vec![
            MethodDesc { name: "Describe$".to_string(), arity: 0, arg_names: vec![], return_type: "String".to_string() },
            MethodDesc { name: "TotalRaces%".to_string(), arity: 0, arg_names: vec![], return_type: "Integer".to_string() },
            MethodDesc { name: "WinRate".to_string(), arity: 0, arg_names: vec![], return_type: "Float".to_string() },
            MethodDesc { name: "Tags$".to_string(), arity: 0, arg_names: vec![], return_type: "String[]".to_string() },
            MethodDesc { name: "Info$".to_string(), arity: 0, arg_names: vec![], return_type: "String".to_string() },
        ],
        examples: vec![
            "DIM r@ AS BMX_RIDER(\"Alice\", 17, \"Expert\", 12, 3)".to_string(),
            "PRINT r@.Describe$()".to_string(),
        ],
    }
}

#[derive(Debug, Clone)]
struct BmxRider {
    name: String,
    age: i32,
    skill_level: String,
    wins: i64,
    losses: i64,
}

impl BmxRider {
    fn win_rate(&self) -> f64 {
        let total = self.wins + self.losses;
        if total <= 0 { 0.0 } else { self.wins as f64 / total as f64 }
    }
}

fn make_str_array(items: Vec<String>) -> Value {
    use basil_bytecode::{ElemType, ArrayObj};
    let dims = vec![items.len()];
    let data = items.into_iter().map(Value::Str).collect::<Vec<_>>();
    let arr = Rc::new(ArrayObj { elem: ElemType::Str, dims, data: RefCell::new(data) });
    Value::Array(arr)
}

impl BasicObject for BmxRider {
    fn type_name(&self) -> &str { "BMX_RIDER" }
    fn get_prop(&self, name: &str) -> Result<Value> {
        match name.to_ascii_uppercase().as_str() {
            "NAME$" => Ok(Value::Str(self.name.clone())),
            "AGE%" => Ok(Value::Int(self.age as i64)),
            "SKILLLEVEL$" => Ok(Value::Str(self.skill_level.clone())),
            "WINS%" => Ok(Value::Int(self.wins)),
            "LOSSES%" => Ok(Value::Int(self.losses)),
            other => Err(BasilError(format!("Unknown property '{}' on BMX_RIDER", other))),
        }
    }
    fn set_prop(&mut self, name: &str, v: Value) -> Result<()> {
        match name.to_ascii_uppercase().as_str() {
            "NAME$" => { self.name = match v { Value::Str(s)=>s, other=>format!("{}", other) }; Ok(()) }
            "AGE%" => { self.age = match v { Value::Int(i)=>i as i32, Value::Num(n)=>n.trunc() as i32, _=>0 }; Ok(()) }
            "SKILLLEVEL$" => { self.skill_level = match v { Value::Str(s)=>s, other=>format!("{}", other) }; Ok(()) }
            "WINS%" => { self.wins = match v { Value::Int(i)=>i, Value::Num(n)=>n.trunc() as i64, _=>0 }; Ok(()) }
            "LOSSES%" => { self.losses = match v { Value::Int(i)=>i, Value::Num(n)=>n.trunc() as i64, _=>0 }; Ok(()) }
            other => Err(BasilError(format!("Unknown property '{}' on BMX_RIDER", other))),
        }
    }
    fn call(&mut self, method: &str, _args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "DESCRIBE$" => Ok(Value::Str(format!("{} ({}{}, {}) — {}W/{}L, {:.1}%", 
                self.name,
                self.age,
                if self.age==1 {" yr"} else {" yrs"},
                self.skill_level,
                self.wins, self.losses, self.win_rate()*100.0))),
            "TOTALRACES%" => Ok(Value::Int(self.wins + self.losses)),
            "WINRATE" => Ok(Value::Num(self.win_rate())),
            "TAGS$" => Ok(make_str_array(vec![self.name.clone(), self.skill_level.clone()])),
            "INFO$" => Ok(Value::Str(format!("Rider {} ({} {})", self.name, self.age, self.skill_level))),
            other => Err(BasilError(format!("Unknown method '{}' on BMX_RIDER", other))),
        }
    }
    fn descriptor(&self) -> ObjectDescriptor { descriptor_static() }
}
