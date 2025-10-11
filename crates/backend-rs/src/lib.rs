//! Rust backend emitter for bcc: turns a tiny IR into a Cargo project.

use std::{fs, path::{Path, PathBuf}};
use sha2::{Digest, Sha256};
use basil_ir::{Module, Instr, Expr};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub enum DepSource {
    CratesIo,                 // use crates.io with pinned versions
    LocalPath(PathBuf),       // use local repo path to runtime crates
    Vendor(PathBuf),          // use crates.io names but provide vendor dir copied into project
}

impl Default for DepSource {
    fn default() -> Self { DepSource::CratesIo }
}

#[derive(Debug, Clone, Default)]
pub struct BuildOptions {
    pub name: Option<String>,
    pub target: Option<String>,
    pub opt_level: Option<u8>,
    pub lto: Option<String>, // off|thin|fat
    pub features: Vec<String>, // effective feature names like "audio","midi" (rt features)
    pub obj_crates: Vec<String>, // e.g. "basil-obj-audio"
    pub emit_project_dir: Option<PathBuf>, // if Some, emit here and don't build
    pub keep_build: bool,
    pub quiet: bool,
    // Dependency source mode (default CratesIo)
    pub dep_source: DepSource,
    // Pinned version string for libbasilrt and basil-obj-* when using crates.io/vendor
    pub pinned_version: String,
}

pub fn compute_hash_key(src: &str, opts: &BuildOptions) -> String {
    let mut hasher = Sha256::new();
    hasher.update(src.as_bytes());
    hasher.update("|features:".as_bytes());
    for f in &opts.features { hasher.update(f.as_bytes()); hasher.update(b","); }
    hasher.update("|objs:".as_bytes());
    for c in &opts.obj_crates { hasher.update(c.as_bytes()); hasher.update(b","); }
    hasher.update("|opt:".as_bytes());
    hasher.update(opts.opt_level.unwrap_or(3).to_le_bytes());
    hasher.update("|lto:".as_bytes());
    hasher.update(opts.lto.clone().unwrap_or_else(|| "thin".into()).as_bytes());
    hasher.update("|deps:".as_bytes());
    match &opts.dep_source {
        DepSource::CratesIo => hasher.update("crates-io".as_bytes()),
        DepSource::LocalPath(p) => { hasher.update("local".as_bytes()); hasher.update(p.to_string_lossy().as_bytes()); }
        DepSource::Vendor(p) => { hasher.update("vendor".as_bytes()); hasher.update(p.to_string_lossy().as_bytes()); }
    }
    hasher.update("|ver:".as_bytes());
    hasher.update(opts.pinned_version.as_bytes());
    let sum = hasher.finalize();
    hex::encode(sum)[..16].to_string()
}

pub struct EmittedProject { pub root: PathBuf, pub main_rs: PathBuf, pub cargo_toml: PathBuf }

pub fn emit_project(base_dir: &Path, src_path: &Path, module: &Module, opts: &BuildOptions) -> std::io::Result<EmittedProject> {
    let hash = compute_hash_key(&format!("{}\n{:?}", src_path.display(), module), opts);
    let root = if let Some(ref dir) = opts.emit_project_dir { dir.clone() } else { base_dir.join(".basil").join("targets").join(&hash) };

    let src_dir = root.join("src");
    fs::create_dir_all(&src_dir)?;

    // Emit Cargo.toml
    let cargo_toml = root.join("Cargo.toml");
    let cargo = render_cargo_toml(opts);
    fs::write(&cargo_toml, cargo)?;

    // Emit src/main.rs
    let main_rs = src_dir.join("main.rs");
    let main_body = render_main_rs(src_path, module);
    fs::write(&main_rs, main_body)?;

    // If vendor mode: copy vendor dir and write .cargo/config.toml
    if let DepSource::Vendor(vendor_src) = &opts.dep_source {
        let vend_dst = root.join("vendor");
        copy_dir_all(vendor_src, &vend_dst)?;
        let cargo_cfg_dir = root.join(".cargo");
        fs::create_dir_all(&cargo_cfg_dir)?;
        let cfg = "[source.crates-io]\nreplace-with = \"vendored-sources\"\n\n[source.vendored-sources]\ndirectory = \"vendor\"\n";
        fs::write(cargo_cfg_dir.join("config.toml"), cfg)?;
    }

    Ok(EmittedProject { root, main_rs, cargo_toml })
}

fn render_cargo_toml(opts: &BuildOptions) -> String {
    let name = opts.name.clone().unwrap_or_else(|| "basil_prog".into());
    let opt = opts.opt_level.unwrap_or(3);
    let lto = opts.lto.clone().unwrap_or_else(|| "thin".into());

    let mut features_list = String::new();
    for (i, f) in opts.features.iter().enumerate() {
        if i > 0 { features_list.push_str(", "); }
        features_list.push('"'); features_list.push_str(f); features_list.push('"');
    }

    let mut obj_lines = String::new();
    for c in &opts.obj_crates {
        obj_lines.push_str(&format!("# {c} = \"={}\"\n", opts.pinned_version));
    }

    let lib_dep = match &opts.dep_source {
        DepSource::CratesIo | DepSource::Vendor(_) => {
            format!("libbasilrt = {{ version = \"={}\", features = [ {} ] }}", opts.pinned_version, features_list)
        }
        DepSource::LocalPath(root) => {
            // Use absolute, sanitized path for Cargo on Windows and Unix
            let abs_root = root.canonicalize().unwrap_or(root.clone());
            let mut p = abs_root.join("crates").join("libbasilrt").to_string_lossy().to_string();
            if cfg!(windows) {
                if let Some(stripped) = p.strip_prefix("\\\\?\\") { p = stripped.to_string(); }
                if let Some(stripped) = p.strip_prefix("//?/") { p = stripped.to_string(); }
                p = p.replace('\\', "/");
            } else {
                p = p.replace('\\', "/");
            }
            format!("libbasilrt = {{ path = \"{}\", features = [ {} ] }}", p, features_list)
        }
    };

    format!(r#"
[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
{lib_dep}
{obj_lines}

[profile.release]
opt-level = {opt}
lto = "{lto}"
codegen-units = 1
panic = "abort"

# Make this a workspace root to avoid inheriting an ancestor workspace
[workspace]
members = []
"#)
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() { fs::create_dir_all(dst)?; }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else if ty.is_file() {
            // Overwrite if exists
            if let Some(parent) = to.parent() { fs::create_dir_all(parent)?; }
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

fn render_main_rs(src_path: &Path, module: &Module) -> String {
    // Render body by walking instructions with a variable mapping context
    let mut body = String::new();
    let mut vars = HashMap::new();
    let mut declared: HashSet<String> = HashSet::new();
    // Pre-scan assignments to decide which variables require mut
    let mut counts: HashMap<String, usize> = HashMap::new();
    collect_assign_counts(&module.main.body, &mut counts);
    let mutated: HashSet<String> = counts.into_iter().filter(|(_, c)| *c > 1).map(|(v, _)| v).collect();
    render_instrs(&module.main.body, &mutated, &mut vars, &mut declared, &mut body, 0);

    format!(r#"#![allow(unused_variables, unused_assignments)]

// AUTOGENERATED by bcc — DO NOT EDIT
// Source: {src}

use libbasilrt as rt;

#[inline(always)]
#[allow(dead_code)]
fn make_val_str(s: String) -> rt::Val {{ rt::Val::Str(rt::Str::from_string(s)) }}

fn basil_main() -> rt::RtResult<()> {{
{body}
    Ok(())
}}

fn main() {{
    if let Err(e) = basil_main() {{
        eprintln!("{{}}", e);
        std::process::exit(1);
    }}
}}
"#, src = src_path.display(), body = body)
}

fn collect_assign_counts(instrs: &Vec<Instr>, counts: &mut HashMap<String, usize>) {
    // Count assignments using a canonical, case-insensitive variable key so that
    // Basil variables like "A%" and "a%" are treated as the same variable.
    let canon = |s: &str| -> String { s.to_ascii_lowercase() };
    for ins in instrs {
        match ins {
            Instr::Assign { var, .. } => {
                let key = canon(var);
                let e = counts.entry(key).or_insert(0);
                *e += 1;
            }
            Instr::If { then_body, else_body, .. } => {
                collect_assign_counts(then_body, counts);
                collect_assign_counts(else_body, counts);
            }
            Instr::For { body, .. } => {
                collect_assign_counts(body, counts);
            }
            _ => {}
        }
    }
}

fn render_instrs(instrs: &Vec<Instr>, mutated: &HashSet<String>, vars: &mut HashMap<String, String>, declared: &mut HashSet<String>, out: &mut String, mut indent: usize) {
    // vars: maps canonical (lowercased) Basil var name -> sanitized Rust ident
    let ind = |n: usize| -> String { " ".repeat(n * 4) };
    let canon = |s: &str| -> String { s.to_ascii_lowercase() };
    for ins in instrs {
        match ins {
            Instr::Print(e) => {
                let s_code = render_expr_as_string(e, vars);
                out.push_str(&format!("{}rt::print(&make_val_str({}))?;\n", ind(indent), s_code));
            }
            Instr::For { var, start, end, step, body } => {
                let rust_var = sanitize_var(var);
                let start_c = render_expr_as_i64(start, vars);
                let end_c = render_expr_as_i64(end, vars);
                let step_c = step.as_ref().map(|e| render_expr_as_i64(e, vars)).unwrap_or_else(|| "1".to_string());
                out.push_str(&format!("{}{{\n", ind(indent)));
                indent += 1;
                out.push_str(&format!("{}let mut {}: i64 = {};\n", ind(indent), rust_var, start_c));
                out.push_str(&format!("{}let end_: i64 = {};\n", ind(indent), end_c));
                out.push_str(&format!("{}let step_: i64 = {};\n", ind(indent), step_c));
                out.push_str(&format!("{}while if step_ >= 0 {{ {} <= end_ }} else {{ {} >= end_ }} {{\n", ind(indent), rust_var, rust_var));
                indent += 1;
                // extend var map with canonical key for the loop var (do not add to declared set)
                let key = canon(var);
                let prev = vars.insert(key.clone(), rust_var.clone());
                render_instrs(body, mutated, vars, declared, out, indent);
                // restore
                if let Some(prev_name) = prev { vars.insert(key, prev_name); } else { vars.remove(&canon(var)); }
                out.push_str(&format!("{}{} = {}.saturating_add(step_);\n", ind(indent), rust_var, rust_var));
                indent -= 1;
                out.push_str(&format!("{}}}\n", ind(indent)));
                indent -= 1;
                out.push_str(&format!("{}}}\n", ind(indent)));
            }
            Instr::Assign { var, expr } => {
                let rust_var = sanitize_var(var);
                let val = render_expr_as_i64(expr, vars);
                let key = canon(var);
                if declared.contains(&rust_var) || vars.contains_key(&key) {
                    out.push_str(&format!("{}{} = {};\n", ind(indent), rust_var, val));
                } else {
                    vars.insert(key.clone(), rust_var.clone());
                    declared.insert(rust_var.clone());
                    let mut_kw = if mutated.contains(&key) { "mut " } else { "" };
                    out.push_str(&format!("{}let {}{}: i64 = {};\n", ind(indent), mut_kw, rust_var, val));
                }
            }
            Instr::If { cond, then_body, else_body } => {
                let c = render_expr_as_bool(cond, vars);
                out.push_str(&format!("{}if {} {{\n", ind(indent), c));
                indent += 1;
                render_instrs(then_body, mutated, vars, declared, out, indent);
                indent -= 1;
                if !else_body.is_empty() {
                    out.push_str(&format!("{}}} else {{\n", ind(indent)));
                    indent += 1;
                    render_instrs(else_body, mutated, vars, declared, out, indent);
                    indent -= 1;
                }
                out.push_str(&format!("{}}}\n", ind(indent)));
            }
            Instr::ExprStmt(e) => {
                // Evaluate for side effects if it's a known call; otherwise ignore
                if let Expr::Call(name, args) = e {
                    if name.as_str() == "AUDIO_PLAY%" {
                        let a0 = render_expr_as_string(&args[0], vars);
                        let a1 = render_expr_as_string(&args[1], vars);
                        out.push_str(&format!("{}let _ = rt::features::daw::audio_play(&{}, &{});\n", ind(indent), a0, a1));
                    } else if name.as_str() == "DAW_STOP" {
                        out.push_str(&format!("{}rt::features::daw::stop();\n", ind(indent)));
                    } else if name.as_str() == "DAW_RESET" {
                        out.push_str(&format!("{}rt::features::daw::reset();\n", ind(indent)));
                    }
                }
            }
        }
    }
}

fn sanitize_var(name: &str) -> String {
    let mut s = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' { s.push(ch.to_ascii_lowercase()); }
        else if ch == '%' || ch == '$' { s.push_str("_v"); }
        else { s.push('_'); }
    }
    if s.is_empty() { s.push_str("v"); }
    if s.chars().next().unwrap().is_ascii_digit() { s.insert(0, '_'); }
    s
}

fn render_expr_as_string(e: &Expr, vars: &HashMap<String, String>) -> String {
    let canon = |s: &str| -> String { s.to_ascii_lowercase() };
    match e {
        Expr::Str(s) => format!("\"{}\".to_string()", escape_str(s)),
        Expr::Int(i) => format!("{}.to_string()", i),
        Expr::Bool(b) => format!("{}.to_string()", b),
        Expr::Var(name) => {
            let v = vars.get(&canon(name)).cloned().unwrap_or_else(|| sanitize_var(name));
            format!("{}.to_string()", v)
        }
        Expr::Add(a, b) => {
            let ac = render_expr_as_string(a, vars);
            let bc = render_expr_as_string(b, vars);
            format!("{{ let mut s = {}; s.push_str(&{}); s }}", ac, bc)
        }
        Expr::Call(name, _args) if name.as_str() == "DAW_ERR$" => {
            "rt::features::daw::get_err()".to_string()
        }
        _ => "String::new()".to_string(),
    }
}

fn render_expr_as_bool(e: &Expr, vars: &HashMap<String, String>) -> String {
    match e {
        Expr::Ne(a, b) => {
            let ac = render_expr_as_i64(a, vars);
            let bc = render_expr_as_i64(b, vars);
            format!("{} != {}", ac, bc)
        }
        _ => "false".to_string(),
    }
}

fn render_expr_as_i64(e: &Expr, vars: &HashMap<String, String>) -> String {
    let canon = |s: &str| -> String { s.to_ascii_lowercase() };
    match e {
        Expr::Int(i) => format!("{}", i),
        Expr::Var(name) => vars.get(&canon(name)).cloned().unwrap_or_else(|| sanitize_var(name)),
        Expr::Call(name, args) if name.as_str() == "AUDIO_PLAY%" => {
            let a0 = render_expr_as_string(&args[0], vars);
            let a1 = render_expr_as_string(&args[1], vars);
            format!("rt::features::daw::audio_play(&{}, &{})", a0, a1)
        }
        Expr::Call(name, args) if name.as_str() == "SYNTH_LIVE%" => {
            let a0 = render_expr_as_string(&args[0], vars);
            let a1 = render_expr_as_string(&args[1], vars);
            let a2 = render_expr_as_i64(&args[2], vars);
            format!("rt::features::daw::synth_live(&{}, &{}, {})", a0, a1, a2)
        }
        // Best-effort fallback for unexpected types
        _ => "0".to_string(),
    }
}

fn escape_str(s: &str) -> String { s.replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r") }
