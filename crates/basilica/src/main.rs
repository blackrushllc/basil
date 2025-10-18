mod config;
mod app;
mod instance;

use eframe::{App as EApp, NativeOptions};

struct EgWrapper { inner: app::BasilicaApp }
impl EApp for EgWrapper {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.inner.ui(ctx);
        ctx.request_repaint();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() >= 2 && args[0] == "--bootstrap" {
        let script = args[1].clone();
        // Load existing or seed config, convert to host pending
        let existing = match config::load_or_seed() {
            Ok(c) => c,
            Err(e) => { eprintln!("Failed to load basilica.json: {}", e); std::process::exit(1); }
        };
        let pending_host = config::to_host_pending(&existing);
        let shared = std::sync::Arc::new(parking_lot::Mutex::new(pending_host));

        // Start a headless Basil runner with BASILICA.MENU enabled
        let opts = basil_embed::RunnerOptions { with_app: false, with_web: false, basilica_menu: Some(shared.clone()), host_tx: None };
        let runner = basil_embed::BasilRunner::spawn(false, opts);
        let _ = runner.tx.send(basil_embed::RunnerCmd::RunFile { mode: basil_embed::RunMode::Run, path: script.clone(), args: None });
        // Drain events until exit
        let mut exit_code: i32 = 0;
        loop {
            match runner.rx.recv() {
                Ok(basil_embed::RunnerEvent::Output(s)) => { print!("{}", s); let _ = std::io::Write::flush(&mut std::io::stdout()); }
                Ok(basil_embed::RunnerEvent::Error(e)) => { eprintln!("{}", e); exit_code = 1; }
                Ok(basil_embed::RunnerEvent::Suspended) => { /* ignore in bootstrap */ }
                Ok(basil_embed::RunnerEvent::Exited) => break,
                Err(_) => { exit_code = 1; break; }
            }
        }
        // If script requested save, write config
        let saved;
        let snapshot;
        {
            let p = shared.lock();
            saved = p.saved;
            snapshot = p.clone();
        }
        if saved {
            let new_cfg = config::from_host_pending(&snapshot);
            if let Err(e) = config::save_atomic(&new_cfg) { eprintln!("Failed to save basilica.json: {}", e); std::process::exit(1); }
            println!("Saved {} CLI items, {} GUI items.", new_cfg.cli_scripts.len(), new_cfg.gui_scripts.len());
            std::process::exit(0);
        } else {
            std::process::exit(exit_code.max(1));
        }
    }

    let cfg = match config::load_or_seed() {
        Ok(cfg) => cfg,
        Err(e) => { eprintln!("Failed to load or seed basilica.json: {}", e); std::process::exit(1); }
    };

    let native_options = NativeOptions::default();
    let _ = eframe::run_native(
        "Basilica",
        native_options,
        Box::new(|_cc| Ok(Box::new(EgWrapper { inner: app::BasilicaApp::new(cfg) }))),
    );
}
