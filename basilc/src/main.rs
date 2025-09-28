/*

 ▄▄▄▄    ██▓    ▄▄▄       ▄████▄   ██ ▄█▀ ██▀███   █    ██   ██████  ██░ ██
▓█████▄ ▓██▒   ▒████▄    ▒██▀ ▀█   ██▄█▒ ▓██ ▒ ██▒ ██  ▓██▒▒██    ▒ ▓██░ ██▒
▒██▒ ▄██▒██░   ▒██  ▀█▄  ▒▓█    ▄ ▓███▄░ ▓██ ░▄█ ▒▓██  ▒██░░ ▓██▄   ▒██▀▀██░
▒██░█▀  ▒██░   ░██▄▄▄▄██ ▒▓▓▄ ▄██▒▓██ █▄ ▒██▀▀█▄  ▓▓█  ░██░  ▒   ██▒░▓█ ░██
░▓█  ▀█▓░██████▒▓█   ▓██▒▒ ▓███▀ ░▒██▒ █▄░██▓ ▒██▒▒▒█████▓ ▒██████▒▒░▓█▒░██▓
░▒▓███▀▒░ ▒░▓  ░▒▒   ▓▒█░░ ░▒ ▒  ░▒ ▒▒ ▓▒░ ▒▓ ░▒▓░░▒▓▒ ▒ ▒ ▒ ▒▓▒ ▒ ░ ▒ ░░▒░▒
▒░▒   ░ ░ ░ ▒  ░ ▒   ▒▒ ░  ░  ▒   ░ ░▒ ▒░  ░▒ ░ ▒░░░▒░ ░ ░ ░ ░▒  ░ ░ ▒ ░▒░ ░
 ░    ░   ░ ░    ░   ▒   ░        ░ ░░ ░   ░░   ░  ░░░ ░ ░ ░  ░  ░   ░  ░░ ░
 ░          ░  ░     ░  ░░ ░      ░  ░      ░        ░           ░   ░  ░  ░
      ░                  ░
Copyright (C) 2026, Blackrush LLC, All Rights Reserved
Created by Erik Lee Olson, Tarpon Springs, Florida
For more information, visit BlackrushDrive.com

MIT License

Copyright (c) 2026 Erik Lee Olson for Blackrush, LLC

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

*/

use std::env;
use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::fs;
use std::path::{Path, PathBuf};

//use std::{env, fs, io, path::Path};
use basil_parser::parse;
use basil_compiler::compile;
use basil_vm::VM;
use basil_lexer::Lexer; // add this near the other use lines




// Map fun aliases → canonical commands
fn canonicalize(cmd: &str) -> &str {
    match cmd.to_ascii_lowercase().as_str() {
        // serious
        "init" => "init",
        "run" => "run",
        "build" => "build",
        "test" => "test",
        "fmt" => "fmt",
        "add" => "add",
        "clean" => "clean",
        "dev" => "dev",
        "serve" => "serve",
        "doc" => "doc",
        // punny
        "seed" => "init",
        "sprout" => "run",
        "harvest" => "build",
        "cultivate" => "test",
        "prune" => "fmt",
        "infuse" => "add",
        "compost" => "clean",
        "steep" => "dev",
        "greenhouse" => "serve",
        "bouquet" => "doc",
        "lex" => "lex",
        "chop" => "lex",   // fun alias
        _ => cmd,
    }
}

fn print_help() {
    println!("Basil CLI (prototype)\n");
    println!("Commands (aliases in parentheses):");
    println!("  init (seed)        Create a new Basil project");
    println!("  run  (sprout)      Parse → compile → run a .basil file");
    println!("  build (harvest)    Build project (stub)");
    println!("  test (cultivate)   Run tests (stub)");
    println!("  fmt  (prune)       Format sources (stub)");
    println!("  add  (infuse)      Add dependency (stub)");
    println!("  clean (compost)    Remove build artifacts (stub)");
    println!("  dev  (steep)       Start dev mode (stub)");
    println!("  serve (greenhouse) Serve local HTTP (stub)");
    println!("  doc  (bouquet)     Generate docs (stub)\n");
    println!("  lex  (chop)        Dump tokens from a .basil file (debug)");
    println!("Usage:");
    println!("  basilc <command> [args]\n");
    println!("Examples:");
    println!("  basilc run examples/hello.basil");
    println!("  basilc sprout examples/hello.basil");
    println!("  basilc init myapp");

}

fn cmd_init(target: Option<String>) -> io::Result<()> {
    let name = target.unwrap_or_else(|| "basil_app".to_string());
    let root = Path::new(&name);
    if root.exists() {
        eprintln!("error: path '{}' already exists", name);
        std::process::exit(1);
    }
    fs::create_dir_all(root.join("src"))?;
    fs::write(root.join("src/main.basil"), "PRINT \"Hello, Basil!\";\n")?;
    let toml = format!(
        "package = \"{}\"\nversion = \"0.0.1\"\nedition = \"2026\"\n\n[dependencies]\n",
        name
    );
    fs::write(root.join("basil.toml"), toml)?;
    println!("Initialized Basil project at ./{}", name);
    Ok(())
}

fn cmd_lex(path: Option<String>) {
    let Some(path) = path else { eprintln!("usage: basilc lex <file.basil>"); std::process::exit(2) };
    let src = std::fs::read_to_string(&path).expect("read file");
    let mut lx = Lexer::new(&src);
    match lx.tokenize() {
        Ok(toks) => {
            for t in toks {
                println!("{:?}\t'{}'\t@{}..{}", t.kind, t.lexeme, t.span.start, t.span.end);
            }
        }
        Err(e) => { eprintln!("lex error: {}", e); std::process::exit(1); }
    }
}

fn cmd_run(path: Option<String>) {
    // Require a path
    let path = match path {
        Some(p) => p,
        None => {
            eprintln!("usage: basilc run <file.basil>");
            std::process::exit(2);
        }
    };

    // Optional: refuse obvious non-source invocations (helps catch /usr/lib/cgi-bin/basil.cgi)
    if !path.ends_with(".basil") {
        eprintln!("Refusing to run a non-.basil file: {}", path);
        std::process::exit(2);
    }

    // Read the source once, with good error messages
    let src = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
            eprintln!("File is not UTF-8 text: {}", path);
            std::process::exit(3);
        }
        Err(e) => {
            eprintln!("Failed to read {}: {}", path, e);
            std::process::exit(1);
        }
    };

    // Parse → compile → run
    match parse(&src) {
        Ok(ast) => match compile(&ast) {
            Ok(prog) => {
                let mut vm = VM::new(prog);
                if let Err(e) = vm.run() {
                    eprintln!("runtime error: {}", e);
                    std::process::exit(1);
                }
            }
            Err(e) => {
                eprintln!("compile error: {}", e);
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("parse error: {}", e);
            std::process::exit(1);
        }
    }
}



/// --- New: mode detection ---

fn is_cgi_invocation() -> bool {
    // Apache/CGI set these; lighttpd/nginx-fastcgi set similar.
    env::var("GATEWAY_INTERFACE").is_ok() && env::var("REQUEST_METHOD").is_ok()
}

/// --- Your existing CLI entry, unchanged logic moved here ---

fn cli_main() {
    // === BEGIN: your old main() body ===
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        print_help();
        return;
    }
    let cmd = canonicalize(&args[0]).to_string();
    args.remove(0);

    match cmd.as_str() {
        "init" => {
            let name = args.get(0).cloned();
            if let Err(e) = cmd_init(name) {
                eprintln!("init error: {}", e);
                std::process::exit(1);
            }
        }
        "run" => {
            cmd_run(args.get(0).cloned());
        }
        "build" | "test" | "fmt" | "add" | "clean" | "dev" | "serve" | "doc" => {
            println!("[stub] '{}' not implemented yet in the prototype", cmd);
        }
        "lex" => { cmd_lex(args.get(0).cloned()); }
        other => {
            eprintln!("unknown command: '{}'\n", other);
            print_help();
            std::process::exit(2);
        }
    }
    // === END: your old main() body ===
}

/// --- New: CGI entrypoint that wraps your CLI 'run' ---

fn cgi_main() {
    // 1) Resolve the Basil script path the request mapped to
    let script_path = resolve_script_path().unwrap_or_else(|| "/var/www/html/index.basil".to_string());

    // let script_path = env::var("SCRIPT_FILENAME")
    //     .or_else(|_| env::var("PATH_TRANSLATED"))
    //     .or_else(|_| env::var("PATH_INFO").map(|p| format!("/var/www{}", p)))
    //     .unwrap_or_else(|_| "/var/www/html/index.basil".to_string());

    if !Path::new(&script_path).exists() {
        println!("Status: 404 Not Found");
        println!("Content-Type: text/plain; charset=utf-8");
        println!();
        println!("Basil file not found: {}", script_path);
        return;
    }

    // 2) Gather request bits
    let method = env::var("REQUEST_METHOD").unwrap_or_else(|_| "GET".into());
    let query  = env::var("QUERY_STRING").unwrap_or_default();
    let ctype  = env::var("CONTENT_TYPE").unwrap_or_default();
    let clen: usize = env::var("CONTENT_LENGTH").ok().and_then(|s| s.parse().ok()).unwrap_or(0);

    let mut body = Vec::with_capacity(clen);
    if clen > 0 {
        let stdin = io::stdin();
        stdin.take(clen as u64).read_to_end(&mut body).ok();
    }

    // 3) Spawn *this* binary in CLI mode to run the script
    //    We force CLI mode so the child doesn't enter cgi_main() again.
    let self_exe = match env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            println!("Status: 500 Internal Server Error");
            println!("Content-Type: text/plain; charset=utf-8");
            println!();
            println!("Failed to locate current executable: {e}");
            return;
        }
    };

    let mut child = match Command::new(self_exe)
        .arg("run")
        .arg(&script_path)
        .env("BASIL_FORCE_MODE", "cli")       // <- prevents recursion
        .env("QUERY_STRING", &query)          // pass through web context
        .env("REQUEST_METHOD", &method)
        .env("CONTENT_TYPE", &ctype)
        .env("CONTENT_LENGTH", clen.to_string())
        .env("SCRIPT_FILENAME", &script_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            println!("Status: 500 Internal Server Error");
            println!("Content-Type: text/plain; charset=utf-8");
            println!();
            println!("Failed to spawn Basil runner: {e}");
            return;
        }
    };

    // 4) Pipe request body to the child (if your Basil runtime wants it)
    if clen > 0 {
        if let Some(mut sin) = child.stdin.take() {
            let _ = sin.write_all(&body);
        }
    }

    // 5) Collect output
    let output = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => {
            println!("Status: 500 Internal Server Error");
            println!("Content-Type: text/plain; charset=utf-8");
            println!();
            println!("Failed to run Basil script: {e}");
            return;
        }
    };

    // Send child's stderr to Apache error log (very helpful)
    if !output.stderr.is_empty() {
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
    }

    // 6) If the Basil program already prints CGI headers, pass them through.
    //    Otherwise, default to HTML.
    let stdout = output.stdout;
    let looks_like_cgi = stdout.starts_with(b"Content-Type:")
        || stdout.starts_with(b"Status:")
        || stdout.windows(2).position(|w| w == b"\r\n").is_some();

    if looks_like_cgi {
        // Assume full CGI response
        io::stdout().write_all(&stdout).ok();
        return;
    }

    // Minimal wrapper headers
    if output.status.success() {
        println!("Status: 200 OK");
        println!("Content-Type: text/html; charset=utf-8");
        println!();
        io::stdout().write_all(&stdout).ok();
    } else {
        println!("Status: 500 Internal Server Error");
        println!("Content-Type: text/plain; charset=utf-8");
        println!();
        io::stdout().write_all(&stdout).ok();
    }
}

// use std::env;
// use std::path::{Path, PathBuf};

fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Ok(h), Ok(l)) = (u8::from_str_radix(&s[i+1..i+2], 16), u8::from_str_radix(&s[i+2..i+3], 16)) {
                out.push((h << 4 | l) as char);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn resolve_script_path() -> Option<String> {
    // 1) Prefer SCRIPT_FILENAME if it points to a .basil file
    if let Ok(sf) = env::var("SCRIPT_FILENAME") {
        if sf.ends_with(".basil") && Path::new(&sf).is_file() {
            return Some(sf);
        }
    }
    // 2) PATH_TRANSLATED is often correct under Action
    if let Ok(pt) = env::var("PATH_TRANSLATED") {
        if pt.ends_with(".basil") && Path::new(&pt).is_file() {
            return Some(pt);
        }
    }
    // 3) Try DOCUMENT_ROOT + PATH_INFO
    if let (Ok(docroot), Ok(pi)) = (env::var("DOCUMENT_ROOT"), env::var("PATH_INFO")) {
        let cand = PathBuf::from(docroot).join(pi.trim_start_matches('/'));
        if cand.extension().and_then(|e| e.to_str()) == Some("basil") && cand.is_file() {
            return Some(cand.to_string_lossy().into_owned());
        }
    }
    // 4) Try DOCUMENT_ROOT + REQUEST_URI (strip query)
    if let (Ok(docroot), Ok(uri)) = (env::var("DOCUMENT_ROOT"), env::var("REQUEST_URI")) {
        let path_part = uri.split('?').next().unwrap_or("");
        let dec = url_decode(path_part);
        let cand = PathBuf::from(docroot).join(dec.trim_start_matches('/'));
        if cand.extension().and_then(|e| e.to_str()) == Some("basil") && cand.is_file() {
            return Some(cand.to_string_lossy().into_owned());
        }
    }
    None
}


/// --- New: tiny dispatcher ---

fn main() {
    // Explicit escape hatch for any subprocess we spawn:
    if env::var("BASIL_FORCE_MODE").ok().as_deref() == Some("cli") {
        cli_main();
        return;
    }

    if is_cgi_invocation() {
        cgi_main();
    } else {
        cli_main();
    }
}
