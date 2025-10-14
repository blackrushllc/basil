use std::cell::RefCell;
use std::rc::Rc;

use basil_bytecode::{ArrayObj, BasicObject, ElemType, MethodDesc, ObjectDescriptor, Value};
use basil_common::{BasilError, Result};

use crate::runtime::RT;

fn make_str_array(items: Vec<String>) -> Value {
    let dims = vec![items.len()];
    let data = items.into_iter().map(Value::Str).collect::<Vec<_>>();
    let arr = Rc::new(ArrayObj { elem: ElemType::Str, dims, data: RefCell::new(data) });
    Value::Array(arr)
}

fn err_msg(op: &str, e: impl std::fmt::Display) -> BasilError { BasilError(format!("SQS.{}: {}", op, e)) }

pub struct SqsObject { pub client: aws_sdk_sqs::Client }
impl SqsObject { pub fn from_client(client: aws_sdk_sqs::Client) -> Self { Self { client } } }

impl BasicObject for SqsObject {
    fn type_name(&self) -> &str { "AWS_SQS" }
    fn get_prop(&self, _name: &str) -> Result<Value> { Err(BasilError("AWS_SQS has no readable properties".into())) }
    fn set_prop(&mut self, _name: &str, _v: Value) -> Result<()> { Err(BasilError("AWS_SQS has no settable properties".into())) }

    fn call(&mut self, method: &str, args: &[Value]) -> Result<Value> {
        match method.to_ascii_uppercase().as_str() {
            "SEND$" => {
                if !(2..=3).contains(&args.len()) { return Err(BasilError("SQS.Send$ expects queue_url$, body$, delay_sec%?".into())); }
                let queue_url = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let body = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let delay = if args.len()>=3 { match &args[2] { Value::Int(i)=> Some(*i as i32), Value::Num(n)=> Some(n.trunc() as i32), _=> None } } else { None };
                let client = self.client.clone();
                let msg_id = RT.block_on(async move {
                    let mut req = client.send_message().queue_url(queue_url).message_body(body);
                    if let Some(d) = delay { req = req.delay_seconds(d); }
                    let out = req.send().await.map_err(|e| err_msg("Send", e))?;
                    Ok::<String, BasilError>(out.message_id.unwrap_or_default())
                })?;
                Ok(Value::Str(msg_id))
            }
            ,"RECEIVE$" => {
                if !(1..=4).contains(&args.len()) { return Err(BasilError("SQS.Receive$ expects queue_url$, max%?, wait_sec%?, vis_timeout_sec%?".into())); }
                let queue_url = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let max = if args.len()>=2 { match &args[1] { Value::Int(i)=> *i as i32, Value::Num(n)=> n.trunc() as i32, _=> 0 } } else { 0 };
                let wait = if args.len()>=3 { match &args[2] { Value::Int(i)=> *i as i32, Value::Num(n)=> n.trunc() as i32, _=> 0 } } else { 0 };
                let vis = if args.len()>=4 { match &args[3] { Value::Int(i)=> *i as i32, Value::Num(n)=> n.trunc() as i32, _=> 0 } } else { 0 };
                let client = self.client.clone();
                let items: Vec<String> = RT.block_on(async move {
                    let mut req = client.receive_message().queue_url(queue_url);
                    if max>0 { req = req.max_number_of_messages(max.min(10)); }
                    if wait>0 { req = req.wait_time_seconds(wait.min(20)); }
                    if vis>0 { req = req.visibility_timeout(vis); }
                    let out = req.send().await.map_err(|e| err_msg("Receive", e))?;
                    let mut arr = Vec::new();
                    if let Some(msgs) = out.messages {
                        for m in msgs {
                            let id = m.message_id.unwrap_or_default();
                            let rh = m.receipt_handle.clone().unwrap_or_default();
                            let body = m.body.unwrap_or_default();
                            let json = serde_json::json!({ "MessageId": id, "ReceiptHandle": rh, "Body": body }).to_string();
                            arr.push(json);
                        }
                    }
                    Ok::<Vec<String>, BasilError>(arr)
                })?;
                Ok(make_str_array(items))
            }
            ,"DELETE$" => {
                if args.len() != 2 { return Err(BasilError("SQS.Delete$ expects queue_url$, receipt_handle$".into())); }
                let queue_url = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let receipt = match &args[1] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let client = self.client.clone();
                RT.block_on(async move {
                    let _ = client.delete_message().queue_url(queue_url).receipt_handle(receipt).send().await.map_err(|e| err_msg("Delete", e))?;
                    Ok::<Value, BasilError>(Value::Int(1))
                })
            }
            ,"PURGE" => {
                if args.len() != 1 { return Err(BasilError("SQS.Purge expects queue_url$".into())); }
                let queue_url = match &args[0] { Value::Str(s)=>s.clone(), other=>format!("{}", other) };
                let client = self.client.clone();
                RT.block_on(async move {
                    let res = client.purge_queue().queue_url(queue_url).send().await;
                    match res { Ok(_)=> Ok(Value::Int(1)), Err(e)=> Err(err_msg("Purge", e)) }
                })
            }
            ,other => Err(BasilError(format!("Unknown method '{}' on AWS_SQS", other)))
        }
    }

    fn descriptor(&self) -> ObjectDescriptor { descriptor_static() }
}

fn descriptor_static() -> ObjectDescriptor {
    ObjectDescriptor {
        type_name: "AWS_SQS".into(),
        version: "0.1".into(),
        summary: "Amazon SQS client for Basil".into(),
        properties: vec![],
        methods: vec![
            MethodDesc { name: "Send$".into(), arity: 3, arg_names: vec!["queue_url$".into(), "body$".into(), "delay_sec%?".into()], return_type: "String (message_id$)".into() },
            MethodDesc { name: "Receive$".into(), arity: 4, arg_names: vec!["queue_url$".into(), "max%?".into(), "wait_sec%?".into(), "vis_timeout_sec%?".into()], return_type: "String[]".into() },
            MethodDesc { name: "Delete$".into(), arity: 2, arg_names: vec!["queue_url$".into(), "receipt_handle$".into()], return_type: "Int (ok%)".into() },
            MethodDesc { name: "Purge".into(), arity: 1, arg_names: vec!["queue_url$".into()], return_type: "Int (ok%)".into() },
        ],
        examples: vec![
            "DIM aws@ AS AWS() : DIM sqs@ = aws@.MakeSQS()".into(),
        ],
    }
}

pub fn register<F: FnMut(&str, crate::TypeInfo)>(reg: &mut F) {
    let factory = |_args: &[Value]| -> Result<Rc<RefCell<dyn BasicObject>>> {
        let ctx = crate::context::AwsContext::default();
        let val = ctx.make_sqs()?;
        match val { Value::Object(o)=> Ok(o), _=> Err(BasilError("unexpected".into())) }
    };
    let descriptor = || descriptor_static();
    let constants = || Vec::<(String, Value)>::new();
    reg("AWS_SQS", crate::TypeInfo { factory, descriptor, constants });
}
