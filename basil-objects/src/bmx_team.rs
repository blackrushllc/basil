use basil_common::{Result, BasilError};
use basil_bytecode::{Value, ObjectDescriptor, PropDesc, MethodDesc, BasicObject};
use std::rc::Rc;
use std::cell::RefCell;

pub fn register(reg: &mut crate::Registry) {
    reg.register("BMX_TEAM", crate::TypeInfo {
        factory: |args| {
            // BMX_TEAM(name$, establishedYear%, [flags%])
            if args.len() < 2 || args.len() > 3 { return Err(BasilError(format!("BMX_TEAM expects 2 or 3 args, got {}", args.len()))); }
            let name = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
            let year = match &args[1] { Value::Int(i)=>*i as i64, Value::Num(n)=>n.trunc() as i64, _=>0 } as i32;
            let flags = if args.len() == 3 { match &args[2] { Value::Int(i)=>*i, Value::Num(n)=>n.trunc() as i64, _=>0 } } else { 0 };
            Ok(Rc::new(RefCell::new(BmxTeam { name, established_year: year, team_wins: 0, team_losses: 0, roster: Vec::new(), is_pro: flags & 1 != 0 })))
        },
        descriptor: descriptor_static,
        constants: || vec![ ("PRO".to_string(), Value::Int(1)), ("NOT_PRO".to_string(), Value::Int(0)) ],
    });
}

fn descriptor_static() -> ObjectDescriptor {
    ObjectDescriptor {
        type_name: "BMX_TEAM".to_string(),
        version: "1.0".to_string(),
        summary: "Represents a BMX team aggregating riders".to_string(),
        properties: vec![
            PropDesc { name: "Name$".to_string(), type_name: "String".to_string(), readable: true, writable: true },
            PropDesc { name: "EstablishedYear%".to_string(), type_name: "Integer".to_string(), readable: true, writable: true },
            PropDesc { name: "TeamWins%".to_string(), type_name: "Integer".to_string(), readable: true, writable: true },
            PropDesc { name: "TeamLosses%".to_string(), type_name: "Integer".to_string(), readable: true, writable: true },
        ],
        methods: vec![
            MethodDesc { name: "AddRider".to_string(), arity: 1, arg_names: vec!["rider@".to_string()], return_type: "Void".to_string() },
            MethodDesc { name: "RemoveRider".to_string(), arity: 1, arg_names: vec!["name$".to_string()], return_type: "Integer".to_string() },
            MethodDesc { name: "WinPct".to_string(), arity: 0, arg_names: vec![], return_type: "Float".to_string() },
            MethodDesc { name: "TopRider".to_string(), arity: 0, arg_names: vec![], return_type: "BMX_RIDER".to_string() },
            MethodDesc { name: "BottomRider".to_string(), arity: 0, arg_names: vec![], return_type: "BMX_RIDER".to_string() },
            MethodDesc { name: "RiderNames$".to_string(), arity: 0, arg_names: vec![], return_type: "String[]".to_string() },
            MethodDesc { name: "RiderDescriptions$".to_string(), arity: 0, arg_names: vec![], return_type: "String[]".to_string() },
            MethodDesc { name: "Info$".to_string(), arity: 0, arg_names: vec![], return_type: "String".to_string() },
        ],
        examples: vec![
            "DIM t@ AS BMX_TEAM(\"Rocket Foxes\", 2015, PRO)".to_string(),
            "t@.AddRider(r@)".to_string(),
        ],
    }
}

#[derive(Clone)]
struct BmxTeam {
    name: String,
    established_year: i32,
    team_wins: i64,
    team_losses: i64,
    roster: Vec<Rc<RefCell<dyn basil_bytecode::BasicObject>>>,
    is_pro: bool,
}

fn make_str_array(items: Vec<String>) -> Value {
    use basil_bytecode::{ElemType, ArrayObj};
    let dims = vec![items.len()];
    let data = items.into_iter().map(Value::Str).collect::<Vec<_>>();
    let arr = Rc::new(ArrayObj { elem: ElemType::Str, dims, data: RefCell::new(data) });
    Value::Array(arr)
}

impl BasicObject for BmxTeam {
    fn type_name(&self) -> &str { "BMX_TEAM" }
    fn get_prop(&self, name: &str) -> Result<Value> {
        match name.to_ascii_uppercase().as_str() {
            "NAME$" => Ok(Value::Str(self.name.clone())),
            "ESTABLISHEDYEAR%" => Ok(Value::Int(self.established_year as i64)),
            "TEAMWINS%" => Ok(Value::Int(self.team_wins)),
            "TEAMLOSSES%" => Ok(Value::Int(self.team_losses)),
            other => Err(BasilError(format!("Unknown property '{}' on BMX_TEAM", other))),
        }
    }
    fn set_prop(&mut self, name: &str, v: Value) -> Result<()> {
        match name.to_ascii_uppercase().as_str() {
            "NAME$" => { self.name = match v { Value::Str(s)=>s, other=>format!("{}", other) }; Ok(()) }
            "ESTABLISHEDYEAR%" => { self.established_year = match v { Value::Int(i)=>i as i32, Value::Num(n)=>n.trunc() as i32, _=>0 }; Ok(()) }
            "TEAMWINS%" => { self.team_wins = match v { Value::Int(i)=>i, Value::Num(n)=>n.trunc() as i64, _=>0 }; Ok(()) }
            "TEAMLOSSES%" => { self.team_losses = match v { Value::Int(i)=>i, Value::Num(n)=>n.trunc() as i64, _=>0 }; Ok(()) }
            other => Err(BasilError(format!("Unknown property '{}' on BMX_TEAM", other))),
        }
    }
    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "ADDRIDER" => {
                if args.len()!=1 { return Err(BasilError("AddRider expects 1 arg".into())); }
                match &args[0] { Value::Object(r) => { self.roster.push(r.clone()); Ok(Value::Null) }, _ => Err(BasilError("AddRider expects a BMX_RIDER object".into())) }
            }
            "REMOVERIDER" => {
                if args.len()!=1 { return Err(BasilError("RemoveRider expects 1 arg".into())); }
                let name = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let before = self.roster.len();
                self.roster.retain(|r| r.borrow().get_prop("Name$").ok().map(|v| match v { Value::Str(s)=>s, _=>String::new() }).unwrap_or_default() != name);
                let removed = before - self.roster.len();
                Ok(Value::Int(if removed>0 {1} else {0}))
            }
            "WINPCT" => {
                let total = self.team_wins + self.team_losses;
                if total <= 0 { Ok(Value::Num(0.0)) } else { Ok(Value::Num(self.team_wins as f64 / total as f64)) }
            }
            "TOPRIDER" => {
                if self.roster.is_empty() { return Err(BasilError("TopRider: roster empty".into())); }
                let mut best_idx = 0usize;
                let mut best_wrp = -1.0;
                let mut best_wins = -1i64;
                let mut best_age = i64::MAX;
                for (i, r) in self.roster.iter().enumerate() {
                    let wr = match r.borrow_mut().call("WinRate", &[]) { Ok(Value::Num(n))=>n, _=>0.0 };
                    let wins = match r.borrow().get_prop("Wins%") { Ok(Value::Int(i))=>i, _=>0 };
                    let age = match r.borrow().get_prop("Age%") { Ok(Value::Int(i))=>i, _=>0 };
                    if wr > best_wrp || (wr == best_wrp && (wins > best_wins || (wins == best_wins && age < best_age))) {
                        best_idx = i; best_wrp = wr; best_wins = wins; best_age = age;
                    }
                }
                Ok(Value::Object(self.roster[best_idx].clone()))
            }
            "BOTTOMRIDER" => {
                if self.roster.is_empty() { return Err(BasilError("BottomRider: roster empty".into())); }
                let mut worst_idx = 0usize;
                let mut worst_wrp = f64::MAX;
                let mut worst_wins = i64::MAX;
                let mut worst_age = -i64::MAX;
                for (i, r) in self.roster.iter().enumerate() {
                    let wr = match r.borrow_mut().call("WinRate", &[]) { Ok(Value::Num(n))=>n, _=>0.0 };
                    let wins = match r.borrow().get_prop("Wins%") { Ok(Value::Int(i))=>i, _=>0 };
                    let age = match r.borrow().get_prop("Age%") { Ok(Value::Int(i))=>i, _=>0 };
                    if wr < worst_wrp || (wr == worst_wrp && (wins < worst_wins || (wins == worst_wins && age > worst_age))) {
                        worst_idx = i; worst_wrp = wr; worst_wins = wins; worst_age = age;
                    }
                }
                Ok(Value::Object(self.roster[worst_idx].clone()))
            }
            "RIDERNAMES$" => {
                let names = self.roster.iter().map(|r| match r.borrow().get_prop("Name$") { Ok(Value::Str(s))=>s, _=>String::new() }).collect::<Vec<_>>();
                Ok(make_str_array(names))
            }
            "RIDERDESCRIPTIONS$" => {
                let descs = self.roster.iter().map(|r| match r.borrow_mut().call("Describe$", &[]) { Ok(Value::Str(s))=>s, _=>String::new() }).collect::<Vec<_>>();
                Ok(make_str_array(descs))
            }
            "INFO$" => Ok(Value::Str(format!("Team {} (est. {}, {}), {} riders", self.name, self.established_year, if self.is_pro {"Pro"} else {"Amateur"}, self.roster.len()))),
            other => Err(BasilError(format!("Unknown method '{}' on BMX_TEAM", other))),
        }
    }
    fn descriptor(&self) -> ObjectDescriptor { descriptor_static() }
}
