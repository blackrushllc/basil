use std::fs;
use std::io::{self, Write};

#[cfg(feature = "obj-ai")]
use basil_objects::ai;

pub fn run_ai_repl() {
    #[cfg(not(feature = "obj-ai"))]
    {
        eprintln!("This binary was not built with --features obj-ai");
        return;
    }
    #[cfg(feature = "obj-ai")]
    {
        let mut system: Option<String> = None;
        let mut model: String = "gpt-4o-mini".to_string();
        let mut last_reply: String = String::new();
        println!("Basil AI REPL (type :quit to exit)");
        let mut input = String::new();
        loop {
            input.clear();
            print!("ai> "); let _ = io::stdout().flush();
            if io::stdin().read_line(&mut input).is_err() { break; }
            let line = input.trim_end();
            if line.is_empty() { continue; }
            if line == ":quit" || line == ":q" { break; }
            if let Some(rest) = line.strip_prefix(":sys ") {
                system = Some(rest.to_string());
                println!("[system set]");
                continue;
            }
            if let Some(rest) = line.strip_prefix(":model ") {
                model = rest.to_string();
                println!("[model set: {}]", model);
                continue;
            }
            if let Some(rest) = line.strip_prefix(":save ") {
                let path = rest.trim();
                match fs::write(path, &last_reply) {
                    Ok(_) => println!("[saved {} bytes to {}]", last_reply.len(), path),
                    Err(e) => eprintln!("save: {}", e),
                }
                continue;
            }
            if let Some(rest) = line.strip_prefix(":explain ") {
                // :explain <file> (ignore :line[-end] for now)
                let parts: Vec<&str> = rest.split(':').collect();
                let path = parts[0];
                match fs::read_to_string(path) {
                    Ok(src) => {
                        let prompt = format!("Explain briefly what this code does (concise):\n\n{}", src);
                        let opts = format!("{{ model: '{}', system: {} }}", model, system.as_deref().map(|s| format!("\"{}\"", s)).unwrap_or("null".to_string()));
                        // stream
                        last_reply = ai_stream(&prompt, Some(&opts));
                        println!(); // newline after streaming
                    }
                    Err(e) => eprintln!("explain: {}", e),
                }
                continue;
            }
            if let Some(rest) = line.strip_prefix(":code ") {
                let ask = rest;
                let prompt = format!("Write Basil BASIC code for: {}\nReturn only a fenced basil code block.", ask);
                let opts = format!("{{ model: '{}', system: {} }}", model, system.as_deref().map(|s| format!("\"{}\"", s)).unwrap_or("null".to_string()));
                last_reply = ai_stream(&prompt, Some(&opts));
                println!();
                continue;
            }
            // default: chat
            let opts = format!("{{ model: '{}', system: {} }}", model, system.as_deref().map(|s| format!("\"{}\"", s)).unwrap_or("null".to_string()));
            last_reply = ai_stream(line, Some(&opts));
            println!();
        }
    }
}

#[cfg(feature = "obj-ai")]
fn ai_stream(prompt: &str, opts: Option<&str>) -> String {
    // Use the same behavior as AI.STREAM: stream tokens and return full text
    // Reuse the AI object directly.
    let mut obj = ai::new_ai();
    let mut o = obj.borrow_mut();
    match o.call("STREAM", &[basil_bytecode::Value::Str(prompt.to_string()), basil_bytecode::Value::Str(opts.unwrap_or("").to_string())]) {
        Ok(v) => match v { basil_bytecode::Value::Str(s)=>s, _=>String::new() },
        Err(_) => String::new(),
    }
}
