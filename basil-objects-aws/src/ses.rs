use std::cell::RefCell;
use std::rc::Rc;

use basil_bytecode::{BasicObject, MethodDesc, ObjectDescriptor, Value};
use basil_common::{BasilError, Result};

use crate::runtime::RT;

fn err_msg(op: &str, e: impl std::fmt::Display) -> BasilError { BasilError(format!("SES.{}: {}", op, e)) }

pub struct SesObject { pub client: aws_sdk_sesv2::Client }
impl SesObject { pub fn from_client(client: aws_sdk_sesv2::Client) -> Self { Self { client } } }

impl BasicObject for SesObject {
    fn type_name(&self) -> &str { "AWS_SES" }
    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("AWS_SES has no readable properties".into())) }
    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("AWS_SES has no settable properties".into())) }

    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "SENDEMAIL" => {
                if !(3..=6).contains(&args.len()) { return Err(BasilError("SES.SendEmail expects to$, subject$, body$, from$?, reply_to$?, is_html%?".into())); }
                let to = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let subject = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let body = match &args[2] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let from = if args.len()>=4 { match &args[3] { Value::Str(s)=>s.clone(), _=>String::new() } } else { String::new() };
                let reply_to = if args.len()>=5 { match &args[4] { Value::Str(s)=> if s.is_empty(){None}else{Some(s.clone())}, _=>None } } else { None };
                let is_html = if args.len()>=6 { match &args[5] { Value::Int(i)=> *i != 0, Value::Num(n)=> n.trunc() as i64 != 0, _=> false } } else { true };
                if from.is_empty() { return Err(BasilError("SES.SendEmail: from$ is required".into())); }
                let client = self.client.clone();
                let msg_id = RT.block_on(async move {
                    use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};
                    let to_addrs: Vec<String> = to.split(',').map(|s| s.trim().to_string()).filter(|s|!s.is_empty()).collect();
                    let dest = Destination::builder().set_to_addresses(Some(to_addrs)).build();
                    let subject_c = Content::builder().data(subject).build().map_err(|e| err_msg("SendEmail(subject)", e))?;
                    let mut b = Body::builder();
                    if is_html {
                        let html_c = Content::builder().data(body.clone()).build().map_err(|e| err_msg("SendEmail(html)", e))?;
                        b = b.html(html_c);
                    } else {
                        let text_c = Content::builder().data(body.clone()).build().map_err(|e| err_msg("SendEmail(text)", e))?;
                        b = b.text(text_c);
                    }
                    let body_built = b.build();
                    let msg = Message::builder().subject(subject_c).body(body_built).build();
                    let email = EmailContent::builder().simple(msg).build();
                    let mut req = client.send_email().destination(dest).content(email).from_email_address(from);
                    if let Some(rt) = reply_to { req = req.set_reply_to_addresses(Some(vec![rt])); }
                    let out = req.send().await.map_err(|e| err_msg("SendEmail", e))?;
                    Ok::<String, BasilError>(out.message_id.unwrap_or_default())
                })?;
                Ok(Value::Str(msg_id))
            }
            ,"SENDRAW$" => {
                if args.len() != 1 { return Err(BasilError("SES.SendRaw$ expects mime$".into())); }
                let mime = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let client = self.client.clone();
                let msg_id = RT.block_on(async move {
                    use aws_sdk_sesv2::types::{RawMessage, EmailContent};
                    let bytes = mime.into_bytes();
                    let raw = RawMessage::builder().data(bytes.into()).build().map_err(|e| err_msg("SendRaw(raw)", e))?;
                    let content = EmailContent::builder().raw(raw).build();
                    let out = client.send_email().content(content).send().await.map_err(|e| err_msg("SendRaw", e))?;
                    Ok::<String, BasilError>(out.message_id.unwrap_or_default())
                })?;
                Ok(Value::Str(msg_id))
            }
            ,other => Err(BasilError(format!("Unknown method '{}' on AWS_SES", other)))
        }
    }

    fn descriptor(&self) -> ObjectDescriptor { descriptor_static() }
}

fn descriptor_static() -> ObjectDescriptor {
    ObjectDescriptor {
        type_name: "AWS_SES".into(),
        version: "0.1".into(),
        summary: "Amazon SES client for Basil".into(),
        properties: vec![],
        methods: vec![
            MethodDesc { name: "SendEmail".into(), arity: 6, arg_names: vec!["to$".into(), "subject$".into(), "body$".into(), "from$?".into(), "reply_to$?".into(), "is_html%?".into()], return_type: "String (message_id$)".into() },
            MethodDesc { name: "SendRaw$".into(), arity: 1, arg_names: vec!["mime$".into()], return_type: "String (message_id$)".into() },
        ],
        examples: vec![
            "DIM aws@ AS AWS() : DIM ses@ = aws@.MakeSES()".into(),
        ],
    }
}

pub fn register<F: FnMut(&str, crate::TypeInfo)>(reg: &mut F) {
    let factory = |_args: &[Value]| -> Result<Rc<RefCell<dyn BasicObject>>> {
        let ctx = crate::context::AwsContext::default();
        let val = ctx.make_ses()?;
        match val { Value::Object(o)=> Ok(o), _=> Err(BasilError("unexpected".into())) }
    };
    let descriptor = || descriptor_static();
    let constants = || Vec::<(String, Value)>::new();
    reg("AWS_SES", crate::TypeInfo { factory, descriptor, constants });
}
