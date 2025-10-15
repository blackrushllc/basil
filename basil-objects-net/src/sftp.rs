use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::rc::Rc;

use basil_bytecode::{ArrayObj, BasicObject, ElemType, MethodDesc, ObjectDescriptor, Value};
use basil_common::{BasilError, Result};

fn make_str_array(items: Vec<String>) -> Value {
    let dims = vec![items.len()];
    let data = items.into_iter().map(Value::Str).collect::<Vec<_>>();
    let arr = Rc::new(ArrayObj { elem: ElemType::Str, dims, data: RefCell::new(data) });
    Value::Array(arr)
}

fn norm_path(p: &str) -> String { p.replace('\\', "/") }

fn err(op: &str, e: impl std::fmt::Display) -> BasilError { BasilError(format!("SFTP.{}: {}", op, e)) }

pub fn register<F: FnMut(&str, crate::TypeInfo)>(reg: &mut F) {
    let descriptor = || descriptor_static();
    let constants = || Vec::<(String, Value)>::new();
    let factory = |args: &[Value]| -> Result<Rc<RefCell<dyn BasicObject>>> {
        // host$, user$, pass$?, keyfile$?, port%?
        if args.len() < 2 { return Err(BasilError("NET_SFTP requires at least host$, user$".into())); }
        let host = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
        let user = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
        let pass = if args.len()>=3 { match &args[2] { Value::Str(s)=>s.clone(), other=>format!("{}", other) } } else { String::new() };
        let keyfile = if args.len()>=4 { match &args[3] { Value::Str(s)=>s.clone(), other=>format!("{}", other) } } else { String::new() };
        let mut port: u16 = 22;
        if args.len()>=5 { port = match &args[4] { Value::Int(i)=> (*i as i64).max(0) as u16, Value::Num(n)=> n.trunc().max(0.0) as u16, _=>22 }; }
        Ok(Rc::new(RefCell::new(NetSftp { host, user, pass, keyfile, port })))
    };
    reg("NET_SFTP", crate::TypeInfo { factory, descriptor, constants });
}

#[derive(Clone)]
struct NetSftp {
    host: String,
    user: String,
    pass: String,
    keyfile: String,
    port: u16,
}

impl NetSftp {
    fn connect(&self) -> Result<(ssh2::Session, ssh2::Sftp)> {
        // Establish TCP
        let addr = format!("{}:{}", self.host, self.port);
        let tcp = TcpStream::connect(&addr).map_err(|e| err("Connect(tcp)", e))?;
        let mut sess = ssh2::Session::new().map_err(|e| err("Connect(session)", e))?;
        sess.set_tcp_stream(tcp);
        sess.handshake().map_err(|e| err("Connect(handshake)", e))?;
        if !self.keyfile.is_empty() {
            let key = Path::new(&self.keyfile);
            let passphrase = if self.pass.is_empty() { None } else { Some(self.pass.as_str()) };
            sess.userauth_pubkey_file(&self.user, None, key, passphrase).map_err(|e| err("Connect(auth pubkey)", e))?;
        } else if !self.pass.is_empty() {
            sess.userauth_password(&self.user, &self.pass).map_err(|e| err("Connect(auth password)", e))?;
        } else {
            return Err(BasilError("SFTP.Connect: missing credentials — provide pass$ or keyfile$".into()));
        }
        if !sess.authenticated() { return Err(BasilError("SFTP.Connect: authentication failed".into())); }
        let sftp = sess.sftp().map_err(|e| err("Connect(sftp)", e))?;
        Ok((sess, sftp))
    }
}

impl BasicObject for NetSftp {
    fn type_name(&self) -> &str { "NET_SFTP" }
    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("NET_SFTP has no readable properties".into())) }
    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("NET_SFTP has no settable properties".into())) }
    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "CONNECT" => {
                // Just verify connection; do not persist session (stateless for simplicity)
                let _ = self.connect()?; // drop immediately
                Ok(Value::Int(1))
            }
            ,"PUT$" => {
                if args.len()!=2 { return Err(BasilError("NET_SFTP.Put$ expects local_file$, remote_path$".into())); }
                let local = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let remote = norm_path(&match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) });
                let (_sess, sftp) = self.connect()?;
                let mut lf = File::open(&local).map_err(|e| err("Put(open local)", e))?;
                let mut rf = sftp.create(Path::new(&remote)).map_err(|e| err("Put(create remote)", e))?;
                let mut buf = [0u8; 64*1024];
                loop {
                    let n = lf.read(&mut buf).map_err(|e| err("Put(read local)", e))?;
                    if n == 0 { break; }
                    rf.write_all(&buf[..n]).map_err(|e| err("Put(write remote)", e))?;
                }
                Ok(Value::Int(1))
            }
            ,"GETTOFILE" => {
                if args.len()!=2 { return Err(BasilError("NET_SFTP.GetToFile expects remote_path$, local_file$".into())); }
                let remote = norm_path(&match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) });
                let local = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let (_sess, sftp) = self.connect()?;
                let mut rf = sftp.open(Path::new(&remote)).map_err(|e| err("GetToFile(open remote)", e))?;
                let mut lf = File::create(&local).map_err(|e| err("GetToFile(create local)", e))?;
                let mut buf = [0u8; 64*1024];
                loop {
                    let n = rf.read(&mut buf).map_err(|e| err("GetToFile(read remote)", e))?;
                    if n == 0 { break; }
                    lf.write_all(&buf[..n]).map_err(|e| err("GetToFile(write local)", e))?;
                }
                Ok(Value::Int(1))
            }
            ,"LIST$" => {
                if args.len()!=1 { return Err(BasilError("NET_SFTP.List$ expects remote_path$".into())); }
                let remote = norm_path(&match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) });
                let (_sess, sftp) = self.connect()?;
                let entries = sftp.readdir(Path::new(&remote)).map_err(|e| err("List$", e))?;
                let mut names: Vec<String> = Vec::new();
                for (p, _stat) in entries {
                    if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                        if name != "." && name != ".." { names.push(name.to_string()); }
                    }
                }
                Ok(make_str_array(names))
            }
            ,"MKDIR" => {
                if args.len()!=1 { return Err(BasilError("NET_SFTP.Mkdir expects remote_path$".into())); }
                let remote = norm_path(&match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) });
                let (_sess, sftp) = self.connect()?;
                sftp.mkdir(Path::new(&remote), 0o755).map_err(|e| err("Mkdir", e))?;
                Ok(Value::Int(1))
            }
            ,"RMDIR" => {
                if args.len()!=1 { return Err(BasilError("NET_SFTP.Rmdir expects remote_path$".into())); }
                let remote = norm_path(&match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) });
                let (_sess, sftp) = self.connect()?;
                sftp.rmdir(Path::new(&remote)).map_err(|e| err("Rmdir", e))?;
                Ok(Value::Int(1))
            }
            ,"DELETE" => {
                if args.len()!=1 { return Err(BasilError("NET_SFTP.Delete expects remote_path$".into())); }
                let remote = norm_path(&match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) });
                let (_sess, sftp) = self.connect()?;
                sftp.unlink(Path::new(&remote)).map_err(|e| err("Delete", e))?;
                Ok(Value::Int(1))
            }
            ,"RENAME" => {
                if args.len()!=2 { return Err(BasilError("NET_SFTP.Rename expects old_path$, new_path$".into())); }
                let oldp = norm_path(&match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) });
                let newp = norm_path(&match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) });
                let (_sess, sftp) = self.connect()?;
                sftp.rename(Path::new(&oldp), Path::new(&newp), None).map_err(|e| err("Rename", e))?;
                Ok(Value::Int(1))
            }
            ,other => Err(BasilError(format!("Unknown method '{}' on NET_SFTP", other)))
        }
    }
    fn descriptor(&self) -> ObjectDescriptor { descriptor_static() }
}

fn descriptor_static() -> ObjectDescriptor {
    ObjectDescriptor {
        type_name: "NET_SFTP".into(),
        version: "0.1".into(),
        summary: "Secure File Transfer over SSH (SFTP)".into(),
        properties: vec![],
        methods: vec![
            MethodDesc { name: "Connect".into(), arity: 0, arg_names: vec![], return_type: "Int (ok%)".into() },
            MethodDesc { name: "Put$".into(), arity: 2, arg_names: vec!["local_file$".into(), "remote_path$".into()], return_type: "Int (ok%)".into() },
            MethodDesc { name: "GetToFile".into(), arity: 2, arg_names: vec!["remote_path$".into(), "local_file$".into()], return_type: "Int (ok%)".into() },
            MethodDesc { name: "List$".into(), arity: 1, arg_names: vec!["remote_path$".into()], return_type: "String[]".into() },
            MethodDesc { name: "Mkdir".into(), arity: 1, arg_names: vec!["remote_path$".into()], return_type: "Int (ok%)".into() },
            MethodDesc { name: "Rmdir".into(), arity: 1, arg_names: vec!["remote_path$".into()], return_type: "Int (ok%)".into() },
            MethodDesc { name: "Delete".into(), arity: 1, arg_names: vec!["remote_path$".into()], return_type: "Int (ok%)".into() },
            MethodDesc { name: "Rename".into(), arity: 2, arg_names: vec!["old_path$".into(), "new_path$".into()], return_type: "Int (ok%)".into() },
        ],
        examples: vec![
            "DIM sftp@ AS NET_SFTP(\"host\", \"user\", \"pass\", \"\", 22)".into(),
        ],
    }
}
