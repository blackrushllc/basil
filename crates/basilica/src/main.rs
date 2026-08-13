mod app;
mod config;
mod instance;

use eframe::{App as EApp, NativeOptions};

struct EgWrapper {
    inner: app::BasilicaApp,
}
impl EApp for EgWrapper {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.inner.ui(ctx);
        ctx.request_repaint();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() >= 1 && args[0] == "--webview-helper" {
        run_webview_helper();
        return;
    }
    if args.len() >= 2 && args[0] == "--bootstrap" {
        let script = args[1].clone();
        // Load existing or seed config, convert to host pending
        let existing = match config::load_or_seed() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to load basilica.json: {}", e);
                std::process::exit(1);
            }
        };
        let pending_host = config::to_host_pending(&existing);
        let shared = std::sync::Arc::new(parking_lot::Mutex::new(pending_host));

        // Start a headless Basil runner with BASILICA.MENU enabled
        let opts = basil_embed::RunnerOptions {
            with_app: false,
            with_web: false,
            basilica_menu: Some(shared.clone()),
            host_tx: None,
        };
        let runner = basil_embed::BasilRunner::spawn(false, opts);
        let _ = runner.tx.send(basil_embed::RunnerCmd::RunFile {
            mode: basil_embed::RunMode::Run,
            path: script.clone(),
            args: None,
        });
        // Drain events until exit
        let mut exit_code: i32 = 0;
        loop {
            match runner.rx.recv() {
                Ok(basil_embed::RunnerEvent::Output(s)) => {
                    print!("{}", s);
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }
                Ok(basil_embed::RunnerEvent::Error(e)) => {
                    eprintln!("{}", e);
                    exit_code = 1;
                }
                Ok(basil_embed::RunnerEvent::Suspended) => { /* ignore in bootstrap */ }
                Ok(basil_embed::RunnerEvent::Exited) => break,
                Err(_) => {
                    exit_code = 1;
                    break;
                }
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
            if let Err(e) = config::save_atomic(&new_cfg) {
                eprintln!("Failed to save basilica.json: {}", e);
                std::process::exit(1);
            }
            println!(
                "Saved {} CLI items, {} GUI items.",
                new_cfg.cli_scripts.len(),
                new_cfg.gui_scripts.len()
            );
            std::process::exit(0);
        } else {
            std::process::exit(exit_code.max(1));
        }
    }

    let cfg = match config::load_or_seed() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Failed to load or seed basilica.json: {}", e);
            std::process::exit(1);
        }
    };

    let native_options = NativeOptions::default();
    let _ = eframe::run_native(
        "Basilica",
        native_options,
        Box::new(|_cc| {
            Ok(Box::new(EgWrapper {
                inner: app::BasilicaApp::new(cfg),
            }))
        }),
    );
}

fn run_webview_helper() {
    use tao::event::{Event, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
    use tao::window::WindowBuilder;
    use wry::WebViewBuilder;

    // Channel for receiving commands from parent (stdin reader thread)
    let (tx_cmd, rx_cmd) = crossbeam_channel::unbounded::<String>();

    // Thread: read JSON lines from stdin and forward to event loop
    std::thread::spawn(move || {
        use std::io::{self, BufRead};
        let stdin = io::stdin();
        let lock = stdin.lock();
        for line in lock.lines() {
            if let Ok(l) = line {
                let _ = tx_cmd.send(l);
            } else {
                break;
            }
        }
    });

    let event_loop: EventLoop<()> = EventLoopBuilder::new().build();

    // Inject bootstrap JS per spec
    let bootstrap_js = r#"
        window.BASIL = {
          send: (obj) => {
            try {
              const s = typeof obj === 'string' ? obj : JSON.stringify(obj);
              if (window.ipc && window.ipc.postMessage) { window.ipc.postMessage(s); }
              else if (window.chrome && window.chrome.webview && window.chrome.webview.postMessage) { window.chrome.webview.postMessage(s); }
            } catch (e) { /* ignore */ }
          },
          receive: (payload) => { /* host may override */ }
        };
        document.addEventListener('click', (e) => {
          const id = e.target && e.target.id;
          if (id) BASIL.send({ event: 'click', id });
        });
    "#;

    // Defer window + webview creation until the event loop is running.
    let mut window_opt: Option<tao::window::Window> = None;
    let mut webview_opt: Option<wry::WebView> = None;

    event_loop.run(move |event, target, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => {
                // Create window/webview on first tick
                if window_opt.is_none() {
                    let window = match WindowBuilder::new()
                        .with_title("Basilica Webview")
                        .build(target)
                    {
                        Ok(w) => w,
                        Err(e) => {
                            eprintln!("[helper] failed to create window: {}", e);
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                    };
                    let wv = match WebViewBuilder::new(&window)
                        .with_initialization_script(bootstrap_js)
                        .with_html("<html><body><div id='root'></div></body></html>")
                        .with_ipc_handler(move |req: wry::http::Request<String>| {
                            let s = req.body().clone();
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                                let event = v
                                    .get("event")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let id = v
                                    .get("id")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                if !event.is_empty() && !id.is_empty() {
                                    // Emit event JSON line to stdout for the parent
                                    let payload = serde_json::json!({"event": event, "id": id});
                                    println!("{}", payload.to_string());
                                }
                            }
                        })
                        .build()
                    {
                        Ok(wv) => wv,
                        Err(e) => {
                            eprintln!("[helper] failed to build webview: {}", e);
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                    };
                    webview_opt = Some(wv);
                    window_opt = Some(window);
                }
                // Drain commands from stdin
                if let Some(wv) = webview_opt.as_ref() {
                    while let Ok(line) = rx_cmd.try_recv() {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                            let cmd = v.get("cmd").and_then(|x| x.as_str()).unwrap_or("");
                            match cmd {
                                "set_html" => {
                                    let html = v.get("html").and_then(|x| x.as_str()).unwrap_or("");
                                    let js = format!(
                                        "document.open();document.write({});document.close();",
                                        serde_json::to_string(html).unwrap_or("\"\"".into())
                                    );
                                    let _ = wv.evaluate_script(&js);
                                }
                                "eval" => {
                                    if let Some(js) = v.get("js").and_then(|x| x.as_str()) {
                                        let _ = wv.evaluate_script(js);
                                    }
                                }
                                _ => { /* ignore */ }
                            }
                        }
                    }
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}
