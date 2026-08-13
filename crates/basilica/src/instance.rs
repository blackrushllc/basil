use std::collections::HashMap;
use std::thread;

use crossbeam_channel as chan;

use crate::config::{MenuKind, MenuMode};
use anyhow::Result;
use basil_embed::{BasilRunner, RunMode, RunnerCmd, RunnerEvent, RunnerOptions};
use basil_host::HostRequest;

// Minimal event payload from webview IPC
#[derive(Debug, Clone)]
pub struct WebEvent {
    pub event: String,
    pub id: String,
}

pub struct ConsoleInstance {
    pub title: String,
    pub output: String,
    pub input: String,
    pub runner: BasilRunner,
    pub rx_evt: chan::Receiver<RunnerEvent>,
    host_rx: chan::Receiver<HostRequest>,
    web_tx: Option<chan::Sender<String>>,          // JS eval
    web_evt_rx: Option<chan::Receiver<WebEvent>>,  // events from webview
    web_routes: HashMap<(String, String), String>, // (event,id) -> label
}

impl ConsoleInstance {
    pub fn new(
        title: String,
        with_web: bool,
        initial: Option<(MenuKind, MenuMode, Option<String>)>,
    ) -> Self {
        let (tx_host, rx_host) = chan::unbounded::<HostRequest>();
        let opts = RunnerOptions {
            with_app: true,
            with_web,
            basilica_menu: None,
            host_tx: Some(tx_host.clone()),
        };
        let start_cli = matches!(initial, Some((MenuKind::Bare, MenuMode::Cli, _)));
        let runner = BasilRunner::spawn(start_cli, opts);
        let rx_evt = runner.rx.clone();
        let mut inst = Self {
            title,
            output: String::new(),
            input: String::new(),
            runner,
            rx_evt,
            host_rx: rx_host,
            web_tx: None,
            web_evt_rx: None,
            web_routes: HashMap::new(),
        };
        if with_web {
            let (tx_js, rx_js) = chan::unbounded::<String>();
            let (tx_ev, rx_ev) = chan::unbounded::<WebEvent>();
            inst.web_tx = Some(tx_js.clone());
            inst.web_evt_rx = Some(rx_ev);
            spawn_webview(
                format!(
                    "<html><body><h3>{}</h3><div id='root'></div></body></html>",
                    inst.title
                ),
                rx_js,
                tx_ev,
            );
        }
        if let Some((kind, mode, path)) = initial {
            match kind {
                MenuKind::Bare => { /* already CLI */ }
                MenuKind::File => {
                    if let Some(p) = path {
                        let rm = match mode {
                            MenuMode::Run => RunMode::Run,
                            MenuMode::Test => RunMode::Test,
                            MenuMode::Cli => RunMode::Cli,
                        };
                        let _ = inst.runner.tx.send(RunnerCmd::RunFile {
                            mode: rm,
                            path: p,
                            args: None,
                        });
                    }
                }
            }
        }
        inst
    }

    pub fn send_line(&self, line: String) {
        let _ = self.runner.tx.send(RunnerCmd::EvalLine(line));
    }

    pub fn update(&mut self) {
        // Drain runner events
        while let Ok(evt) = self.rx_evt.try_recv() {
            match evt {
                RunnerEvent::Output(s) => self.output.push_str(&s),
                RunnerEvent::Error(e) => {
                    self.output.push_str(&format!("[error] {}\n", e));
                }
                RunnerEvent::Suspended => {
                    self.output.push_str("[suspended]\n");
                }
                RunnerEvent::Exited => {
                    self.output.push_str("[exited]\n");
                }
            }
        }
        // Drain host requests (APP/WEB)
        while let Ok(req) = self.host_rx.try_recv() {
            match req {
                HostRequest::AppAlert(msg) => {
                    self.output.push_str(&format!("[ALERT] {}\n", msg));
                }
                HostRequest::AppStartAnim => {
                    self.output.push_str("[ANIM START]\n");
                }
                HostRequest::AppStopAnim => {
                    self.output.push_str("[ANIM STOP]\n");
                }
                HostRequest::WebSetHtml(html) => {
                    if let Some(tx) = &self.web_tx {
                        let _ = tx.send(format!("__SET_HTML__\n{}", html));
                    }
                }
                HostRequest::WebEval(js) => {
                    if let Some(tx) = &self.web_tx {
                        let _ = tx.send(js);
                    }
                }
                HostRequest::WebOn { event, id, label } => {
                    // Register mapping for future dispatch
                    self.web_routes
                        .insert((event.clone(), id.clone()), label.clone());
                    self.output.push_str(&format!(
                        "[WEB.ON] event={} id={} label={}\n",
                        event, id, label
                    ));
                }
            }
        }
        // Drain web events (from IPC)
        if let Some(rx) = &self.web_evt_rx {
            while let Ok(ev) = rx.try_recv() {
                let key = (ev.event.clone(), ev.id.clone());
                if let Some(label) = self.web_routes.get(&key) {
                    self.output.push_str(&format!(
                        "[WEB.EVENT] {}:{} -> {}\n",
                        ev.event, ev.id, label
                    ));
                    // Future: self.runner.tx.send(RunnerCmd::_DispatchWeb { event: ev.event, id: ev.id }).ok();
                } else {
                    self.output.push_str(&format!(
                        "[WEB.EVENT] {}:{} (no handler)\n",
                        ev.event, ev.id
                    ));
                }
            }
        }
    }
}

#[cfg_attr(windows, allow(unreachable_code))]
fn spawn_webview(
    initial_html: String,
    rx_js: chan::Receiver<String>,
    tx_event: chan::Sender<WebEvent>,
) {
    // On Windows, offload the WebView to a helper process whose main thread owns Tao/Wry
    #[cfg(windows)]
    {
        if let Err(e) = spawn_webview_helper_process(initial_html.clone(), rx_js, tx_event) {
            eprintln!("[webview-helper] failed to start: {}", e);
        }
        return;
    }

    // Non-Windows: keep the in-process Wry thread approach
    thread::spawn(move || {
        use tao::event::{Event, WindowEvent};
        use tao::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
        use tao::window::WindowBuilder;
        use wry::WebViewBuilder;

        eprintln!("[webview] thread start");

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
        eprintln!("[webview] entering event loop (deferred create)");
        let mut window_opt: Option<tao::window::Window> = None;
        let mut webview_opt: Option<wry::WebView> = None;
        let initial_html_buf = initial_html;

        event_loop.run(move |event, target, control_flow| {
            *control_flow = ControlFlow::Poll;
            match event {
                Event::MainEventsCleared => {
                    // Create window/webview on first tick
                    if window_opt.is_none() {
                        eprintln!("[webview] creating window...");
                        let window = match WindowBuilder::new()
                            .with_title("Basilica Webview")
                            .build(target)
                        {
                            Ok(w) => w,
                            Err(e) => {
                                eprintln!("[webview] failed to create window: {}", e);
                                *control_flow = ControlFlow::Exit;
                                return;
                            }
                        };
                        eprintln!("[webview] window created");
                        eprintln!("[webview] building webview...");
                        let tx_event2 = tx_event.clone();
                        let wv = match WebViewBuilder::new(&window)
                            .with_initialization_script(bootstrap_js)
                            .with_html(initial_html_buf.clone())
                            .with_ipc_handler(move |req: wry::http::Request<String>| {
                                let s = req.body().clone();
                                let mut ev = None;
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
                                        ev = Some(WebEvent { event, id });
                                    }
                                }
                                if let Some(e) = ev {
                                    let _ = tx_event2.send(e);
                                }
                            })
                            .build()
                        {
                            Ok(wv) => wv,
                            Err(e) => {
                                eprintln!("[webview] failed to build webview: {}", e);
                                *control_flow = ControlFlow::Exit;
                                return;
                            }
                        };
                        eprintln!("[webview] webview created; entering run loop");
                        webview_opt = Some(wv);
                        window_opt = Some(window);
                    }
                    // Drain JS queue
                    if let Some(wv) = webview_opt.as_ref() {
                        while let Ok(code) = rx_js.try_recv() {
                            if let Some(rest) = code.strip_prefix("__SET_HTML__\n") {
                                let js = format!(
                                    "document.open();document.write({});document.close();",
                                    serde_json::to_string(rest).unwrap_or("\"\"".into())
                                );
                                let _ = wv.evaluate_script(&js);
                            } else {
                                let _ = wv.evaluate_script(&code);
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
    });
}

#[cfg(windows)]
fn spawn_webview_helper_process(
    initial_html: String,
    rx_js: chan::Receiver<String>,
    tx_event: chan::Sender<WebEvent>,
) -> Result<()> {
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};

    // Locate helper executable next to basilica.exe
    let exe = std::env::current_exe()?;
    let dir = exe
        .parent()
        .ok_or_else(|| anyhow::anyhow!("no parent dir for current_exe"))?;
    let helper_path = dir.join("basilica-webview-helper.exe");

    let mut cmd = if helper_path.exists() {
        Command::new(helper_path)
    } else {
        // Fallback: launch the main binary in helper mode
        let exe2 = std::env::current_exe()?;
        let mut c = Command::new(exe2);
        c.arg("--webview-helper");
        c
    };
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow::anyhow!("no stdin for helper"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("no stdout for helper"))?;
    let stderr = child.stderr.take();

    // Send initial HTML
    let init_msg = serde_json::json!({"cmd":"set_html", "html": initial_html});
    let mut init_line = serde_json::to_string(&init_msg)?;
    init_line.push('\n');
    stdin.write_all(init_line.as_bytes())?;
    stdin.flush()?;

    // Thread: forward JS/HTML commands to helper stdin
    let writer2 = std::sync::Arc::new(parking_lot::Mutex::new(stdin));
    let writer3 = writer2.clone();
    thread::spawn(move || {
        while let Ok(code) = rx_js.recv() {
            let payload = if let Some(rest) = code.strip_prefix("__SET_HTML__\n") {
                serde_json::json!({"cmd":"set_html", "html": rest})
            } else {
                serde_json::json!({"cmd":"eval", "js": code})
            };
            if let Ok(mut s) = serde_json::to_string(&payload) {
                s.push('\n');
                let mut w = writer3.lock();
                let _ = w.write_all(s.as_bytes());
                let _ = w.flush();
            }
        }
    });

    // Thread: read events from helper stdout and forward to Basilica instance
    let tx2 = tx_event.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let Ok(line) = line else {
                break;
            };
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
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
                    let _ = tx2.send(WebEvent { event, id });
                }
            }
        }
    });

    // Thread: read helper stderr to our stderr for diagnostics
    if let Some(stderr) = stderr {
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(l) = line {
                    eprintln!("[webview-helper] {}", l);
                }
            }
        });
    }

    // Detach child process by forgetting it; pipes kept alive by threads
    std::mem::forget(child);

    Ok(())
}
