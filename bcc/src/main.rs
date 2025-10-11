use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use basil_frontend::parse_program;
use basil_ir::lower_to_ir;
use backend_rs::{emit_project, BuildOptions, DepSource};

fn print_help() {
    println!("bcc aot <input.basil> [options]\n\nOptions:\n  -o <outdir>                Output dir for final exe (unused; prints project path)\n  --name <prog>              Package/binary name\n  --features <spec>          @auto (default) | @all | obj-audio,obj-midi,...\n  --target <triple>          Rust target triple\n  --opt <0|1|2|3>            Optimize level (default 3)\n  --lto <off|thin|fat>       Link-time optimization (default thin)\n  --emit-project <dir>       Emit Cargo project only, don’t build\n  --dep-source <mode>        crates-io (default) | local | vendor\n  --local-runtime <dir>      Repo root containing crates/libbasilrt (for --dep-source local)\n  --vendor-dir <dir>         Directory containing a cargo vendor bundle (for --dep-source vendor)\n  --keep-build               Keep temp build directory\n  --quiet                    Less output\n  -h, --help                 Show this help\n");
}

fn main() {
    let mut args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" { print_help(); return; }
    let cmd = args.remove(0);
    if cmd != "aot" { eprintln!("error: unknown command '{}'. Use 'bcc aot <file>'.", cmd); std::process::exit(2); }
    if args.is_empty() { eprintln!("error: missing <input.basil>"); std::process::exit(2); }

    let input = args.remove(0);
    let input_path = PathBuf::from(&input);
    let src = match fs::read_to_string(&input_path) { Ok(s) => s, Err(e) => { eprintln!("error: {}", e); std::process::exit(1); } };

    // Defaults
    let mut name: Option<String> = None;
    let mut features_spec: Option<String> = None; // default @auto
    let mut target: Option<String> = None;
    let mut opt_level: Option<u8> = None;
    let mut lto: Option<String> = None;
    let mut emit_project_dir: Option<PathBuf> = None;
    let mut keep_build = false;
    let mut quiet = false;

    // Dependency/source selection
    let mut dep_source_choice: Option<String> = None; // crates-io | local | vendor
    let mut local_runtime_root: Option<PathBuf> = None;
    let mut vendor_dir: Option<PathBuf> = None;

    // Parse flags
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" => { i+=1; /* planned: output dir for exe */ i+=1; },
            "--name" => { i+=1; name = args.get(i).cloned(); i+=1; },
            "--features" => { i+=1; features_spec = args.get(i).cloned(); i+=1; },
            "--target" => { i+=1; target = args.get(i).cloned(); i+=1; },
            "--opt" => { i+=1; opt_level = args.get(i).and_then(|s| s.parse::<u8>().ok()); i+=1; },
            "--lto" => { i+=1; lto = args.get(i).cloned(); i+=1; },
            "--emit-project" => { i+=1; emit_project_dir = args.get(i).map(|s| PathBuf::from(s)); i+=1; },
            "--dep-source" => { i+=1; dep_source_choice = args.get(i).cloned(); i+=1; },
            "--local-runtime" => { i+=1; local_runtime_root = args.get(i).map(|s| PathBuf::from(s)); i+=1; },
            "--vendor-dir" => { i+=1; vendor_dir = args.get(i).map(|s| PathBuf::from(s)); i+=1; },
            "--keep-build" => { keep_build = true; i+=1; },
            "--quiet" => { quiet = true; i+=1; },
            other => { eprintln!("warning: unknown option '{}' (ignored)", other); i+=1; }
        }
    }

    // Parse via shared frontend
    let program = match parse_program(&src) {
        Ok(p) => p,
        Err(e) => { eprintln!("parse error: {}", e); std::process::exit(1); }
    };

    // Feature detection
    let auto_features = autodetect_features(&src);
    let curated_default = vec!["audio".to_string(), "midi".to_string(), "daw".to_string(), "term".to_string()];
    let (rt_features, obj_crates) = match features_spec.as_deref() {
        None | Some("@auto") => {
            let set = to_set(curated_default.iter().cloned().chain(auto_features.clone()));
            let rt_vec: Vec<String> = set.iter().cloned().collect();
            (rt_vec, map_rt_to_crates(&set))
        }
        Some("@all") => {
            let all = vec!["audio".to_string(), "midi".to_string(), "daw".to_string(), "term".to_string()];
            (all.clone(), map_rt_to_crates(&to_set(all.into_iter())))
        }
        Some(list) => {
            let objs = list.split(|c| c==',' || c==' ').filter(|s| !s.is_empty());
            let mut rt = Vec::new(); let mut objc = Vec::new();
            for o in objs { if let Some((r, oc)) = map_obj_to_rt(o) { rt.push(r); objc.push(oc); } }
            let rt = to_set(rt.into_iter()).into_iter().collect();
            let objc = to_set(objc.into_iter()).into_iter().collect();
            (rt, objc)
        }
    };

    // Early validation: if src refers to AUDIO_/MIDI_/DAW_/TERM_ but feature missing
    if let Some(miss) = first_missing_required_feature(&src, &rt_features) {
        eprintln!("error: {} requires feature '{}'\nhelp: Add '#USE {}' or run with: --features obj-{}",
            miss.hint, miss.required_obj, miss.suggest_use, miss.cli_name);
        std::process::exit(1);
    }

    // Lower to IR
    let module = lower_to_ir(&program);

    // Resolve dependency source mode
    let dep_source = match dep_source_choice.as_deref() {
        Some("local") => {
            let root = local_runtime_root.clone().unwrap_or_else(|| std::env::current_dir().expect("cwd"));
            DepSource::LocalPath(root)
        }
        Some("vendor") => {
            let vd = vendor_dir.clone().unwrap_or_else(|| {
                eprintln!("error: --dep-source vendor requires --vendor-dir <dir>");
                std::process::exit(2);
            });
            DepSource::Vendor(vd)
        }
        Some("crates-io") | None => {
            if dep_source_choice.is_none() {
                // If user provided vendor-dir without dep-source, assume vendor
                if let Some(vd) = vendor_dir.clone() { DepSource::Vendor(vd) } else if let Some(root) = local_runtime_root.clone() { DepSource::LocalPath(root) } else { DepSource::CratesIo }
            } else {
                DepSource::CratesIo
            }
        }
        Some(other) => {
            eprintln!("warning: unknown --dep-source '{}', defaulting to crates-io", other);
            DepSource::CratesIo
        }
    };

    // Emit project
    let mut opts = BuildOptions::default();
    opts.name = name;
    opts.target = target.clone();
    opts.opt_level = opt_level;
    opts.lto = lto;
    opts.features = rt_features.clone();
    opts.obj_crates = obj_crates.clone();
    opts.emit_project_dir = emit_project_dir.clone();
    opts.keep_build = keep_build;
    opts.quiet = quiet;
    opts.dep_source = dep_source;
    opts.pinned_version = "0.1.0".to_string();

    let base_dir = std::env::current_dir().expect("cwd");
    let emitted = match emit_project(&base_dir, &input_path, &module, &opts) {
        Ok(p) => p,
        Err(e) => { eprintln!("error: failed to emit project: {}", e); std::process::exit(1); }
    };

    if emit_project_dir.is_some() {
        if !quiet { println!("project written to {}", emitted.root.display()); }
        return;
    }

    // Build with cargo
    let mut cmd = Command::new("cargo");
    cmd.arg("build").arg("--release").current_dir(&emitted.root);
    // Use --locked only when appropriate: for crates-io/vendor sources if lock exists; never for local path
    let have_lock = emitted.root.join("Cargo.lock").exists();
    let use_locked = have_lock && !matches!(opts.dep_source, DepSource::LocalPath(_));
    if use_locked { cmd.arg("--locked"); }
    if let Some(t) = target { cmd.arg("--target").arg(t); }
    // Offline when using vendor
    let is_vendor = matches!(opts.dep_source, DepSource::Vendor(_));
    if is_vendor { cmd.arg("--offline"); }
    // Keep build outputs inside the emitted project
    cmd.env("CARGO_TARGET_DIR", emitted.root.join("target"));
    if !quiet { println!("building with Cargo in {}", emitted.root.display()); }
    let status = match cmd.status() { Ok(s) => s, Err(e) => { eprintln!("error: failed to run cargo: {}", e); std::process::exit(1); } };
    if !status.success() { eprintln!("error: cargo build failed"); std::process::exit(1); }

    if !quiet { println!("ok: project at {}", emitted.root.display()); }
    // Determine built executable path inside the emitted project
    let bin_name = opts.name.clone().unwrap_or_else(|| "basil_prog".to_string());
    let bin_dir = if let Some(tgt) = &opts.target { emitted.root.join("target").join(tgt).join("release") } else { emitted.root.join("target").join("release") };
    let built_exe = if cfg!(windows) { bin_dir.join(format!("{}.exe", bin_name)) } else { bin_dir.join(&bin_name) };

    // Move the executable into the current directory and rename to match source file name
    let base_dir = std::env::current_dir().expect("cwd");
    let src_stem = input_path.file_stem().and_then(|s| s.to_str()).unwrap_or("prog");
    let out_name = if cfg!(windows) { format!("{}.exe", src_stem) } else { src_stem.to_string() };
    let out_path = base_dir.join(out_name);

    // Overwrite if it exists
    if out_path.exists() { let _ = fs::remove_file(&out_path); }

    // Try rename (move). If it fails (e.g., cross-filesystem), fall back to copy then remove
    let moved_ok = fs::rename(&built_exe, &out_path).is_ok();
    if !moved_ok {
        match fs::copy(&built_exe, &out_path) {
            Ok(_) => { let _ = fs::remove_file(&built_exe); }
            Err(e) => { eprintln!("error: failed to write executable to {}: {}", out_path.display(), e); std::process::exit(1); }
        }
    }

    // On Unix, ensure the output file is executable (chmod 755)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&out_path) {
            let mut perm = meta.permissions();
            perm.set_mode(0o755);
            let _ = fs::set_permissions(&out_path, perm);
        }
    }

    if !quiet { println!("executable: {}", out_path.display()); }
}

fn to_set<I: Iterator<Item=String>>(it: I) -> std::collections::BTreeSet<String> {
    let mut s = std::collections::BTreeSet::new(); for x in it { s.insert(x); } s
}

fn map_rt_to_crates(rt: &std::collections::BTreeSet<String>) -> Vec<String> {
    let mut v = Vec::new();
    for r in rt {
        match r.as_str() {
            "audio" => v.push("basil-obj-audio".to_string()),
            "midi"  => v.push("basil-obj-midi".to_string()),
            "daw"   => v.push("basil-obj-daw".to_string()),
            "term"  => v.push("basil-obj-term".to_string()),
            _ => {}
        }
    }
    v
}

fn map_obj_to_rt(obj: &str) -> Option<(String,String)> {
    match obj {
        "obj-audio" => Some(("audio".into(), "basil-obj-audio".into())),
        "obj-midi"  => Some(("midi".into(),  "basil-obj-midi".into())),
        "obj-daw"   => Some(("daw".into(),   "basil-obj-daw".into())),
        "obj-term"  => Some(("term".into(),  "basil-obj-term".into())),
        _ => None,
    }
}

#[derive(Debug)]
struct MissingFeature { hint: String, required_obj: String, suggest_use: &'static str, cli_name: &'static str }

fn first_missing_required_feature(src: &str, rt_features: &Vec<String>) -> Option<MissingFeature> {
    let has = |f: &str| rt_features.iter().any(|x| x == f);
    if src.contains("AUDIO_") && !has("audio") { return Some(MissingFeature { hint: "AUDIO_* symbol".into(), required_obj: "obj-audio".into(), suggest_use: "AUDIO", cli_name: "audio" }); }
    if src.contains("MIDI_")  && !has("midi")  { return Some(MissingFeature { hint: "MIDI_* symbol".into(),  required_obj: "obj-midi".into(),  suggest_use: "MIDI",  cli_name: "midi"  }); }
    if src.contains("DAW_")   && !has("daw")   { return Some(MissingFeature { hint: "DAW_* symbol".into(),   required_obj: "obj-daw".into(),   suggest_use: "DAW",   cli_name: "daw"   }); }
    if src.contains("TERM_")  && !has("term")  { return Some(MissingFeature { hint: "TERM_* symbol".into(),  required_obj: "obj-term".into(),  suggest_use: "TERM",  cli_name: "term"  }); }
    None
}

fn autodetect_features(src: &str) -> Vec<String> {
    let mut v = Vec::new();
    for line in src.lines() {
        let t = line.trim();
        if t.starts_with("#USE ") {
            let uses = t[5..].to_ascii_lowercase();
            if uses.contains("audio") { v.push("audio".into()); }
            if uses.contains("midi")  { v.push("midi".into()); }
            if uses.contains("daw")   { v.push("daw".into()); }
            if uses.contains("term")  { v.push("term".into()); }
        }
    }
    if src.contains("AUDIO_") { v.push("audio".into()); }
    if src.contains("MIDI_")  { v.push("midi".into()); }
    if src.contains("DAW_")   { v.push("daw".into()); }
    if src.contains("TERM_")  { v.push("term".into()); }
    // Dedup
    let set = to_set(v.into_iter());
    set.into_iter().collect()
}
