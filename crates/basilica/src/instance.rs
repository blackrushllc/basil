use std::collections::HashMap;
use std::thread;

use crossbeam_channel as chan;

use crate::config::{MenuKind, MenuMode};
use basil_embed::{BasilRunner, RunnerCmd, RunnerEvent, RunMode, RunnerOptions};
use basil_host::HostRequest;

// Minimal event payload from webview IPC
#[derive(Debug, Clone)]
pub struct WebEvent { pub event: String, pub id: String }


pub struct ConsoleInstance {
    pub title: String,
    pub output: String,
    pub input: String,
    pub runner: BasilRunner,
    pub rx_evt: chan::Receiver<RunnerEvent>,
    host_rx: chan::Receiver<HostRequest>,
    web_tx: Option<chan::Sender<String>>, // JS eval
    web_evt_rx: Option<chan::Receiver<WebEvent>>, // events from webview
    web_routes: HashMap<(String,String), String>, // (event,id) -> label
}

impl ConsoleInstance {
    pub fn new(title: String, with_web: bool, initial: Option<(MenuKind, MenuMode, Option<String>)>) -> Self {
        let (tx_host, rx_host) = chan::unbounded::<HostRequest>();
        let opts = RunnerOptions { with_app: true, with_web, basilica_menu: None, host_tx: Some(tx_host.clone()) };
        let start_cli = matches!(initial, Some((MenuKind::Bare, MenuMode::Cli, _)));
        let runner = BasilRunner::spawn(start_cli, opts);
        let rx_evt = runner.rx.clone();
        let mut inst = Self { title, output: String::new(), input: String::new(), runner, rx_evt, host_rx: rx_host, web_tx: None, web_evt_rx: None, web_routes: HashMap::new() };
        if with_web {
            let (tx_js, rx_js) = chan::unbounded::<String>();
            let (tx_ev, rx_ev) = chan::unbounded::<WebEvent>();
            inst.web_tx = Some(tx_js.clone());
            inst.web_evt_rx = Some(rx_ev);
            spawn_webview(format!("<html><body><h3>{}</h3><div id='root'></div></body></html>", inst.title), rx_js, tx_ev);
        }
        if let Some((kind, mode, path)) = initial {
            match kind {
                MenuKind::Bare => { /* already CLI */ }
                MenuKind::File => {
                    if let Some(p) = path {
                        let rm = match mode { MenuMode::Run=>RunMode::Run, MenuMode::Test=>RunMode::Test, MenuMode::Cli=>RunMode::Cli };
                        let _ = inst.runner.tx.send(RunnerCmd::RunFile { mode: rm, path: p, args: None });
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
                RunnerEvent::Error(e) => { self.output.push_str(&format!("[error] {}\n", e)); },
                RunnerEvent::Suspended => { self.output.push_str("[suspended]\n"); },
                RunnerEvent::Exited => { self.output.push_str("[exited]\n"); },
            }
        }
        // Drain host requests (APP/WEB)
        while let Ok(req) = self.host_rx.try_recv() {
            match req {
                HostRequest::AppAlert(msg) => { self.output.push_str(&format!("[ALERT] {}\n", msg)); }
                HostRequest::AppStartAnim => { self.output.push_str("[ANIM START]\n"); }
                HostRequest::AppStopAnim => { self.output.push_str("[ANIM STOP]\n"); }
                HostRequest::WebSetHtml(html) => {
                    if let Some(tx) = &self.web_tx { let _ = tx.send(format!("__SET_HTML__\n{}", html)); }
                }
                HostRequest::WebEval(js) => { if let Some(tx) = &self.web_tx { let _ = tx.send(js); } }
                HostRequest::WebOn { event, id, label } => {
                    // Register mapping for future dispatch
                    self.web_routes.insert((event.clone(), id.clone()), label.clone());
                    self.output.push_str(&format!("[WEB.ON] event={} id={} label={}\n", event, id, label));
                }
            }
        }
        // Drain web events (from IPC)
        if let Some(rx) = &self.web_evt_rx {
            while let Ok(ev) = rx.try_recv() {
                let key = (ev.event.clone(), ev.id.clone());
                if let Some(label) = self.web_routes.get(&key) {
                    self.output.push_str(&format!("[WEB.EVENT] {}:{} -> {}\n", ev.event, ev.id, label));
                    // Future: self.runner.tx.send(RunnerCmd::_DispatchWeb { event: ev.event, id: ev.id }).ok();
                } else {
                    self.output.push_str(&format!("[WEB.EVENT] {}:{} (no handler)\n", ev.event, ev.id));
                }
            }
        }
    }
}

fn spawn_webview(initial_html: String, rx_js: chan::Receiver<String>, tx_event: chan::Sender<WebEvent>) {
    thread::spawn(move || {
        use tao::event::{Event, WindowEvent};
        use tao::event_loop::{ControlFlow, EventLoop};
        use tao::window::WindowBuilder;
        use wry::WebViewBuilder;

        // On Windows, ensure this thread is initialized as STA for WebView2
        #[cfg(windows)]
        unsafe {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
        }

        let event_loop: EventLoop<()> = EventLoop::new();

        let window = match WindowBuilder::new().with_title("Basilica Webview").build(&event_loop) {
            Ok(w) => w,
            Err(e) => { eprintln!("[webview] failed to create window: {}", e); return; }
        };

        // Inject bootstrap JS per spec
        let bootstrap_js = r#"
            window.BASIL = {
              send: (obj) => {
                try { window.ipc.postMessage(typeof obj === 'string' ? obj : JSON.stringify(obj)); }
                catch (e) { /* ignore */ }
              },
              receive: (payload) => { /* host may override */ }
            };
            document.addEventListener('click', (e) => {
              const id = e.target && e.target.id;
              if (id) BASIL.send({ event: 'click', id });
            });
        "#;

        let webview = match WebViewBuilder::new(&window)
            .with_initialization_script(bootstrap_js)
            .with_html(initial_html)
            .with_ipc_handler(move |req: wry::http::Request<String>| {
                            let s = req.body().clone();
                // s is a string; try to parse JSON {event,id}
                let mut ev = None;
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                    let event = v.get("event").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let id = v.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    if !event.is_empty() && !id.is_empty() { ev = Some(WebEvent { event, id }); }
                }
                if let Some(e) = ev { let _ = tx_event.send(e); }
            })
            .build() {
            Ok(wv) => wv,
            Err(e) => { eprintln!("[webview] failed to build webview: {}", e); return; }
        };

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            match event {
                Event::MainEventsCleared => {
                    // Drain JS queue
                    while let Ok(code) = rx_js.try_recv() {
                        if let Some(rest) = code.strip_prefix("__SET_HTML__\n") {
                            let js = format!("document.open();document.write({});document.close();", serde_json::to_string(rest).unwrap_or("\"\"".into()));
                            let _ = webview.evaluate_script(&js);
                        } else {
                            let _ = webview.evaluate_script(&code);
                        }
                    }
                }
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => {}
            }
        });
    });
}
