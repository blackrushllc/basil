use std::{env, fs, io, path::Path};
use basil_lexer::{Lexer, TokenKind};

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
        _ => cmd,
    }
}

fn print_help() {
    println!("Basil CLI (prototype)\n");
    println!("Commands (aliases in parentheses):");
    println!("  init (seed)        Create a new Basil project");
    println!("  run  (sprout)      Tokenize & run (v0: tokenize) a .basil file");
    println!("  build (harvest)    Build project (stub)");
    println!("  test (cultivate)   Run tests (stub)");
    println!("  fmt  (prune)       Format sources (stub)");
    println!("  add  (infuse)      Add dependency (stub)");
    println!("  clean (compost)    Remove build artifacts (stub)");
    println!("  dev  (steep)       Start dev mode (stub)");
    println!("  serve (greenhouse) Serve local HTTP (stub)");
    println!("  doc  (bouquet)     Generate docs (stub)\n");
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

fn cmd_run(path: Option<String>) {
    let Some(path) = path else { eprintln!("usage: basilc run <file.basil>"); std::process::exit(2) };
    let src = fs::read_to_string(&path).expect("read file");
    let mut lex = Lexer::new(&src);
    match lex.tokenize() {
        Ok(tokens) => {
            for t in tokens {
                println!("{:?}\t'{}'\t@{}..{}", t.kind, t.lexeme, t.span.start, t.span.end);
            }
        }
        Err(e) => {
            eprintln!("lex error: {}", e);
            std::process::exit(1);
        }
    }
}

fn main() {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" { print_help(); return; }
    let cmd = canonicalize(&args[0]).to_string();
    args.remove(0);

    match cmd.as_str() {
        "init" => {
            let name = args.get(0).cloned();
            if let Err(e) = cmd_init(name) { eprintln!("init error: {}", e); std::process::exit(1); }
        }
        "run" => { cmd_run(args.get(0).cloned()); }
        "build" | "test" | "fmt" | "add" | "clean" | "dev" | "serve" | "doc" => {
            println!("[stub] '{}' not implemented yet in the prototype", cmd);
        }
        other => {
            eprintln!("unknown command: '{}'\n", other);
            print_help();
            std::process::exit(2);
        }
    }
}
