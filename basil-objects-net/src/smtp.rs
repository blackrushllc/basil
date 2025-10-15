use std::cell::RefCell;
use std::rc::Rc;

use basil_bytecode::{BasicObject, MethodDesc, ObjectDescriptor, Value};
use basil_common::{BasilError, Result};

use lettre::message::{Mailbox, Message, SinglePart, header::ContentType};
use lettre::{AsyncSmtpTransport, Tokio1Executor, transport::smtp::authentication::Credentials};
use lettre::AsyncTransport;
use lettre::address::Envelope;

use crate::runtime::TOKIO_RT;

fn err(op: &str, e: impl std::fmt::Display) -> BasilError { BasilError(format!("SMTP.{}: {}", op, e)) }


pub fn register<F: FnMut(&str, crate::TypeInfo)>(reg: &mut F) {
    let descriptor = || descriptor_static();
    let constants = || Vec::<(String, Value)>::new();
    let factory = |args: &[Value]| -> Result<Rc<RefCell<dyn BasicObject>>> {
        // host$, user$?, pass$?, port%?, tls_mode$?
        if args.len() < 1 { return Err(BasilError("MAIL_SMTP requires at least host$".into())); }
        let host = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
        let user = if args.len()>=2 { match &args[1] { Value::Str(s)=>s.clone(), _=>String::new() } } else { String::new() };
        let pass = if args.len()>=3 { match &args[2] { Value::Str(s)=>s.clone(), _=>String::new() } } else { String::new() };
        let mut port: u16 = 587;
        if args.len()>=4 { port = match &args[3] { Value::Int(i)=> (*i as i64).max(0) as u16, Value::Num(n)=> n.trunc().max(0.0) as u16, _=>587 }; }
        let tls_mode = if args.len()>=5 { match &args[4] { Value::Str(s)=> s.to_ascii_lowercase(), other=> format!("{}", other).to_ascii_lowercase() } } else { "starttls".to_string() };
        Ok(Rc::new(RefCell::new(SmtpObj { host, user, pass, port, tls_mode, transport: None })))
    };
    reg("MAIL_SMTP", crate::TypeInfo { factory, descriptor, constants });
}

#[derive(Clone)]
struct SmtpObj {
    host: String,
    user: String,
    pass: String,
    port: u16,
    tls_mode: String,
    transport: Option<AsyncSmtpTransport<Tokio1Executor>>,
}

impl SmtpObj {
    fn ensure_transport(&mut self) -> Result<()> {
        if self.transport.is_some() { return Ok(()); }
        let mut builder = match self.tls_mode.as_str() {
            "tls" => lettre::AsyncSmtpTransport::<Tokio1Executor>::relay(&self.host).map_err(|e| err("Builder(tls)", e))?,
            "plain" => lettre::AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&self.host),
            _ => lettre::AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&self.host).map_err(|e| err("Builder(starttls)", e))?,
        };
        builder = builder.port(self.port);
        if !self.user.is_empty() {
            let creds = Credentials::new(self.user.clone(), self.pass.clone());
            builder = builder.credentials(creds);
        }
        let transport = builder.build();
        self.transport = Some(transport);
        Ok(())
    }
}

impl BasicObject for SmtpObj {
    fn type_name(&self) -> &str { "MAIL_SMTP" }
    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("MAIL_SMTP has no readable properties".into())) }
    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("MAIL_SMTP has no settable properties".into())) }
    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "SENDEMAIL" => {
                if !(args.len()==3 || args.len()==5) { return Err(BasilError("MAIL_SMTP.SendEmail expects to$, subject$, body$, from$?, is_html%?".into())); }
                let to = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let subject = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let body = match &args[2] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let from = if args.len()>=4 { match &args[3] { Value::Str(s)=>s.clone(), _=> String::new() } } else { String::new() };
                let is_html = if args.len()==5 { match &args[4] { Value::Int(i)=> *i != 0, Value::Num(n)=> *n != 0.0, _=> false } } else { true };
                self.ensure_transport()?;
                let from_addr = if !from.is_empty() { from } else { self.user.clone() };
                if from_addr.is_empty() { return Err(BasilError("SMTP.SendEmail: missing from$ and no user$ provided".into())); }
                let message = {
                    let mb = Message::builder().from(from_addr.parse::<Mailbox>().map_err(|e| err("From", e))?)
                        .to(to.parse::<Mailbox>().map_err(|e| err("To", e))?)
                        .subject(subject);
                    let part = if is_html { SinglePart::builder().header(ContentType::TEXT_HTML).body(body) } else { SinglePart::builder().header(ContentType::TEXT_PLAIN).body(body) };
                    mb.singlepart(part).map_err(|e| err("BuildMessage", e))?
                };
                let tx = self.transport.as_ref().unwrap().clone();
                let resp = TOKIO_RT.block_on(async move { tx.send(message).await }).map_err(|e| err("SendEmail", e))?;
                // Try to return message id if visible; otherwise ok%
                let msg = resp.message().collect::<Vec<_>>().join(" ");
                if msg.to_ascii_lowercase().contains("id=") { Ok(Value::Str(msg)) } else { Ok(Value::Int(1)) }
            }
            ,"SENDRAW$" => {
                if args.len()!=1 { return Err(BasilError("MAIL_SMTP.SendRaw$ expects mime$".into())); }
                let raw = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                self.ensure_transport()?;
                // Parse minimal Envelope from headers
                let mut from_addr = String::new();
                let mut to_addrs: Vec<String> = Vec::new();
                for line in raw.lines() {
                    let l = line.trim();
                    let lcase = l.to_ascii_lowercase();
                    if lcase.starts_with("from:") { from_addr = l[5..].trim().trim_matches('<').trim_matches('>').to_string(); }
                    if lcase.starts_with("to:") { let rest = l[3..].trim(); for p in rest.split(',') { let a = p.trim().trim_matches('<').trim_matches('>').to_string(); if !a.is_empty() { to_addrs.push(a); } } }
                    if l.is_empty() { break; }
                }
                if from_addr.is_empty() || to_addrs.is_empty() { return Err(BasilError("SMTP.SendRaw$: MIME must include From: and To: headers".into())); }
                let env = Envelope::new(
                    Some(from_addr.parse().map_err(|e| err("From(env)", e))?),
                    to_addrs.into_iter().map(|s| s.parse().unwrap()).collect()
                ).map_err(|e| err("Envelope", e))?;
                let tx = self.transport.as_ref().unwrap().clone();
                let bytes = raw.replace("\n", "\r\n").into_bytes();
                let resp = TOKIO_RT.block_on(async move { tx.send_raw(&env, &bytes).await }).map_err(|e| err("SendRaw$", e))?;
                let msg = resp.message().collect::<Vec<_>>().join(" ");
                if msg.to_ascii_lowercase().contains("id=") { Ok(Value::Str(msg)) } else { Ok(Value::Int(1)) }
            }
            ,"MAKEMIME$" => {
                // MakeMime$(from$, to$, subject$, text_body$?, html_body$?, attach_path$?)
                if args.len()<3 || args.len()>6 { return Err(BasilError("MAIL_SMTP.MakeMime$ expects from$, to$, subject$, text_body$?, html_body$?, attach_path$?".into())); }
                let from = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let to = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let subject = match &args[2] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let text_body = if args.len()>=4 { match &args[3] { Value::Str(s)=>Some(s.clone()), _=>None } } else { None };
                let html_body = if args.len()>=5 { match &args[4] { Value::Str(s)=>Some(s.clone()), _=>None } } else { None };
                let attach = if args.len()>=6 { match &args[5] { Value::Str(s)=>Some(s.clone()), _=>None } } else { None };
                // Build a very simple MIME string (not robust for all cases). Prefer SendEmail for simple messages.
                let mut headers = Vec::new();
                headers.push(format!("From: {}", from));
                headers.push(format!("To: {}", to));
                headers.push(format!("Subject: {}", subject));
                headers.push("MIME-Version: 1.0".to_string());
                let body = if let Some(html) = html_body {
                    headers.push("Content-Type: text/html; charset=UTF-8".to_string());
                    html
                } else if let Some(text) = text_body {
                    headers.push("Content-Type: text/plain; charset=UTF-8".to_string());
                    text
                } else {
                    String::new()
                };
                if let Some(_att) = attach { /* Omitted for simplicity in Phase 1 */ }
                let mut mime = headers.join("\r\n");
                mime.push_str("\r\n\r\n");
                mime.push_str(&body);
                Ok(Value::Str(mime))
            }
            ,other => Err(BasilError(format!("Unknown method '{}' on MAIL_SMTP", other)))
        }
    }
    fn descriptor(&self) -> ObjectDescriptor { descriptor_static() }
}

fn descriptor_static() -> ObjectDescriptor {
    ObjectDescriptor {
        type_name: "MAIL_SMTP".into(),
        version: "0.1".into(),
        summary: "SMTP mail sender (STARTTLS/TLS/plain via rustls)".into(),
        properties: vec![],
        methods: vec![
            MethodDesc { name: "SendEmail".into(), arity: 5, arg_names: vec!["to$".into(), "subject$".into(), "body$".into(), "from$?".into(), "is_html%?".into()], return_type: "Int (ok%) or String (message_id$)".into() },
            MethodDesc { name: "SendRaw$".into(), arity: 1, arg_names: vec!["mime$".into()], return_type: "Int (ok%) or String (message_id$)".into() },
            MethodDesc { name: "MakeMime$".into(), arity: 6, arg_names: vec!["from$".into(), "to$".into(), "subject$".into(), "text_body$?".into(), "html_body$?".into(), "attach_path$?".into()], return_type: "String (mime$)".into() },
        ],
        examples: vec![
            "DIM smtp@ AS MAIL_SMTP(\"smtp.example.com\", \"user@example.com\", \"apppass\", 587, \"starttls\")".into(),
        ],
    }
}
