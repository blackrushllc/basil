use std::io::{BufRead, BufReader};
use std::thread;

use crossbeam_channel as chan;
use serde_json::Value as Json;

fn main() {
    // Windows: ensure STA for WebView2
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Com::{
            CoInitializeEx, COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE,
        };
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE);
    }

    let (tx_cmd, rx_cmd) = chan::unbounded::<HelperCmd>();

    // Spawn stdin reader thread (JSON-Lines protocol)
    thread::spawn(move || {
        let stdin = std::io::stdin();
        let reader = BufReader::new(stdin.lock());
        for line in reader.lines() {
            let Ok(line) = line else {
                break;
            };
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<Json>(&line) {
                if let Some(cmd) = parse_cmd(&v) {
                    let _ = tx_cmd.send(cmd);
                }
            }
        }
    });

    // Build Tao event loop and window on this main thread
    use tao::event::{Event, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoop};
    use tao::window::WindowBuilder;
    use wry::WebViewBuilder;

    let event_loop: EventLoop<()> = EventLoop::new();

    let window = match WindowBuilder::new()
        .with_title("Basilica Webview")
        .build(&event_loop)
    {
        Ok(w) => w,
        Err(e) => {
            eprintln!("[helper] failed to create window: {}", e);
            return;
        }
    };

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

    let webview = match WebViewBuilder::new(&window)
        .with_initialization_script(bootstrap_js)
        .with_html("<html><body><div id='root'></div></body></html>")
        .with_ipc_handler(move |req: wry::http::Request<String>| {
            let s = req.body().clone();
            // forward as JSON line to stdout
            println!("{}", s);
        })
        .build()
    {
        Ok(wv) => wv,
        Err(e) => {
            eprintln!("[helper] failed to build webview: {}", e);
            return;
        }
    };

    // Apply commands on each tick
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => {
                while let Ok(cmd) = rx_cmd.try_recv() {
                    match cmd {
                        HelperCmd::SetHtml(html) => {
                            let js = format!(
                                "document.open();document.write({});document.close();",
                                serde_json::to_string(&html).unwrap_or("\"\"".into())
                            );
                            let _ = webview.evaluate_script(&js);
                        }
                        HelperCmd::Eval(js) => {
                            let _ = webview.evaluate_script(&js);
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

#[derive(Debug)]
enum HelperCmd {
    SetHtml(String),
    Eval(String),
}

fn parse_cmd(v: &Json) -> Option<HelperCmd> {
    let cmd = v.get("cmd")?.as_str()?.to_ascii_lowercase();
    match cmd.as_str() {
        "set_html" => {
            let html = v.get("html")?.as_str()?.to_string();
            Some(HelperCmd::SetHtml(html))
        }
        "eval" => {
            let js = v.get("js")?.as_str()?.to_string();
            Some(HelperCmd::Eval(js))
        }
        _ => None,
    }
}
