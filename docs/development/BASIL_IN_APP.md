The Basil "starter app”: You can ship a tiny, cross-platform Rust GUI that:

* shows a normal GUI (buttons, dialogs, menus)
* lets users open **one or more embedded Basil REPL windows**, each with its own isolated VM
* exposes **host APIs** from the parent app to Basil via a mini “mod” (so Basil scripts can drive the GUI—open files, start animations, etc.)

Below is a lean, pragmatic skeleton using **eframe/egui** (pure Rust, simple to ship). Swap in Iced/Slint/Tauri later if you want native widgets or webview.

---

# Architecture at a glance

* **GUI shell (eframe/egui)**

    * Main window with a menu: **File → New Basil Window**
    * A tabbed (or multi-window) **Basil Console** widget per VM instance
    * “Busy-box” samples: a button counter, a color picker, an “Open File…” dialog (`rfd` crate), a “Start/Stop animation” toggle

* **Basil embedding layer**

    * A trait `BasilEngine` to abstract your real VM (`basilc` internals).
    * Each console owns a Basil instance running in a background task, with a pair of channels: **stdin → VM** and **stdout/stderr → UI**.
    * On `quit`/`exit`, the VM task ends and the console tab closes.

* **Host→Basil bridge (Feature Object style)**

    * Register a **host mod** (e.g., `obj-app`) that exposes functions to Basil:

        * `APP.OPEN_FILE$()` → opens a file dialog and returns a path
        * `APP.START_ANIM%()` / `APP.STOP_ANIM%()`
        * `APP.ALERT%("message")`
    * Internally these call back into the GUI through a thread-safe `HostApi` trait.

---

# Workspace layout

```
basil-gui/
├─ Cargo.toml
├─ crates/
│  ├─ app/                 # GUI shell (eframe)
│  ├─ basil_embed/         # thin adapter to your Basil VM
│  └─ host_api/            # parent-app APIs exposed to Basil (obj-app)
└─ assets/                 # optional icons
```

**Cargo.toml (workspace)**

```toml
[workspace]
members = ["crates/app", "crates/basil_embed", "crates/host_api"]

[workspace.dependencies]
eframe = "0.27"
egui = "0.27"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
crossbeam-channel = "0.5"
rfd = "0.14"         # native file dialogs
anyhow = "1"
thiserror = "1"
parking_lot = "0.12"

# Replace with your real Basil crates as needed:
basil-vm = { path = "../path/to/your/basil/vm", optional = true }
```

---

# 1) Parent-app → Basil embedding (crates/basil_embed)

```rust
// crates/basil_embed/src/lib.rs
use crossbeam_channel::{unbounded, Receiver, Sender};
use anyhow::Result;
use std::thread;

pub struct Io {
    pub to_vm: Sender<String>,      // user input lines → VM
    pub from_vm: Receiver<String>,  // VM output lines → UI
}

pub trait HostApi: Send + Sync + 'static {
    fn open_file(&self) -> Option<String>;
    fn start_anim(&self);
    fn stop_anim(&self);
    fn alert(&self, msg: &str);
}

pub trait BasilEngine: Send + 'static {
    /// Feed a single line (REPL-style).
    fn feed_line(&mut self, line: &str) -> Result<()>;
    /// Called periodically (or on lines) to drain output, returning zero or more lines.
    fn drain_output(&mut self) -> Vec<String>;
    /// Graceful shutdown.
    fn shutdown(&mut self);
}

/// Replace this stub with your real VM adapter.
/// E.g., construct a VM, register obj-app, run a REPL loop, etc.
pub struct StubBasil {
    buf: Vec<String>,
    host: std::sync::Arc<dyn HostApi>,
}

impl StubBasil {
    pub fn new(host: std::sync::Arc<dyn HostApi>) -> Self { Self { buf: vec![], host } }
}

impl BasilEngine for StubBasil {
    fn feed_line(&mut self, line: &str) -> Result<()> {
        let l = line.trim();
        match l {
            "help" => self.buf.push("Commands: help, open, alert, start, stop, exit".into()),
            "open" => {
                if let Some(p) = self.host.open_file() {
                    self.buf.push(format!("Selected: {p}"));
                } else {
                    self.buf.push("Canceled.".into());
                }
            }
            "alert" => { self.host.alert("Hello from Basil!"); }
            "start" => { self.host.start_anim(); self.buf.push("Anim started".into()); }
            "stop" => { self.host.stop_anim(); self.buf.push("Anim stopped".into()); }
            "exit" | "quit" => { self.buf.push("[[EXIT]]".into()); }
            other => self.buf.push(format!("echo: {other}")),
        }
        Ok(())
    }
    fn drain_output(&mut self) -> Vec<String> { std::mem::take(&mut self.buf) }
    fn shutdown(&mut self) {}
}

pub struct BasilRunner {
    pub io: Io,
    handle: Option<thread::JoinHandle<()>>,
}

impl BasilRunner {
    pub fn spawn<E: BasilEngine + 'static>(
        mut engine: E
    ) -> Self {
        let (tx_in, rx_in) = unbounded::<String>();
        let (tx_out, rx_out) = unbounded::<String>();

        let handle = thread::spawn(move || {
            tx_out.send("Basil REPL ready. Type 'help' or 'exit'.".into()).ok();
            while let Ok(line) = rx_in.recv() {
                if engine.feed_line(&line).is_ok() {
                    for o in engine.drain_output() {
                        let exit = o.contains("[[EXIT]]");
                        tx_out.send(o).ok();
                        if exit { return; }
                    }
                }
            }
        });

        Self { io: Io { to_vm: tx_in, from_vm: rx_out }, handle: Some(handle) }
    }
}
```

> When you swap `StubBasil` with your real VM: construct a Basil instance, register your `obj-app` functions (see below), and forward input lines to the VM’s “exec line / REPL” entrypoint.

---

# 2) Host API exposed as a Basil “mod” (crates/host_api)

In your real integration, you’ll register host callbacks with the Basil VM (like any Feature Object). Here we define a trait the GUI owns; the Basil adapter calls it.

```rust
// crates/host_api/src/lib.rs
use std::sync::{Arc};
use parking_lot::Mutex;

#[derive(Default)]
pub struct AppState {
    pub anim_running: bool,
}

pub trait HostBindings {
    fn open_file(&self) -> Option<String>;
    fn start_anim(&self);
    fn stop_anim(&self);
    fn alert(&self, msg: &str);
}

pub struct HostBridge {
    pub state: Arc<Mutex<AppState>>,
    pub ui_callbacks: UiCallbacks,
}

pub struct UiCallbacks {
    pub open_dialog: Box<dyn Fn() -> Option<String> + Send + Sync>,
    pub alert_msg:   Box<dyn Fn(&str) + Send + Sync>,
    pub set_anim:    Box<dyn Fn(bool) + Send + Sync>,
}

impl HostBindings for HostBridge {
    fn open_file(&self) -> Option<String> { (self.ui_callbacks.open_dialog)() }
    fn start_anim(&self) { (self.ui_callbacks.set_anim)(true); }
    fn stop_anim(&self)  { (self.ui_callbacks.set_anim)(false); }
    fn alert(&self, msg: &str) { (self.ui_callbacks.alert_msg)(msg); }
}
```

Your **Basil VM adapter** will accept an `Arc<dyn HostApi>` (or this `HostBindings`) and register functions like `APP.OPEN_FILE$`, `APP.START_ANIM%`, etc., that call into these closures.

---

# 3) GUI shell (crates/app)

```rust
// crates/app/src/main.rs
use eframe::{egui, NativeOptions};
use crossbeam_channel::TryRecvError;
use std::sync::{Arc};
use host_api::{HostBridge, UiCallbacks, AppState};
use basil_embed::{BasilRunner, StubBasil, BasilEngine};

fn main() -> eframe::Result<()> {
    let native_options = NativeOptions::default();
    eframe::run_native(
        "Basil GUI Starter",
        native_options,
        Box::new(|_cc| Box::<GuiApp>::default()),
    )
}

#[derive(Default)]
struct GuiApp {
    consoles: Vec<BasilConsole>,
    clicks: u32,             // busy-box counter
    anim: bool,              // pretend “animation running” flag
    alert_text: Option<String>,
}

struct BasilConsole {
    title: String,
    input_buf: String,
    log: Vec<String>,
    io: basil_embed::Io,
}

impl GuiApp {
    fn open_basil_console(&mut self) {
        // Construct host bridge callbacks for this app
        let app_state = Arc::new(parking_lot::Mutex::new(AppState::default()));
        let host = Arc::new(HostShim {
            open_dialog: Box::new(|| rfd::FileDialog::new().pick_file().map(|p| p.display().to_string())),
            alert_fn:    Box::new(|_|{}), // replaced below after we have &mut self
            set_anim_fn: Box::new(|_|{}), // replaced below
        });

        // Spin up a Basil runner with the stub engine (replace with real engine)
        let engine = StubBasil::new(host.clone());
        let runner = BasilRunner::spawn(engine);

        let idx = self.consoles.len();
        self.consoles.push(BasilConsole {
            title: format!("Basil {}", idx + 1),
            input_buf: String::new(),
            log: vec![],
            io: runner.io,
        });
    }
}

impl eframe::App for GuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Menu bar
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Basil Window").clicked() {
                        self.open_basil_console();
                        ui.close_menu();
                    }
                    if ui.button("Quit").clicked() {
                        std::process::exit(0);
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About…").clicked() {
                        self.alert_text = Some("Basil GUI Starter — demo".into());
                        ui.close_menu();
                    }
                });
            });
        });

        // Busy-box demo area
        egui::SidePanel::left("left").show(ctx, |ui| {
            ui.heading("Busy-box");
            if ui.button("Click me").clicked() { self.clicks += 1; }
            ui.label(format!("Clicks: {}", self.clicks));

            if ui.button(if self.anim { "Stop anim" } else { "Start anim" }).clicked() {
                self.anim = !self.anim;
            }
            if ui.button("Open File…").clicked() {
                if let Some(p) = rfd::FileDialog::new().pick_file() {
                    self.alert_text = Some(format!("You chose: {}", p.display()));
                }
            }
        });

        // Basil consoles as tabs
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::widgets::global_dark_light_mode_switch(ui);
            egui::ScrollArea::both().show(ui, |ui| {
                egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), ui.id().with("consoles"), true)
                    .show_header(ui, |ui| ui.heading("Basil Consoles"))
                    .body(|ui| {
                        for i in (0..self.consoles.len()).collect::<Vec<_>>() {
                            let mut close_me = false;
                            egui::Window::new(&self.consoles[i].title)
                                .open(&mut !close_me)
                                .show(ctx, |ui| {
                                    // Drain VM output
                                    loop {
                                        match self.consoles[i].io.from_vm.try_recv() {
                                            Ok(line) => self.consoles[i].log.push(line),
                                            Err(TryRecvError::Empty) => break,
                                            Err(TryRecvError::Disconnected) => { close_me = true; break; }
                                        }
                                    }
                                    // Log view
                                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                                        for l in &self.consoles[i].log { ui.monospace(l); }
                                    });
                                    // Input line
                                    let r = egui::TextEdit::singleline(&mut self.consoles[i].input_buf)
                                        .hint_text("type 'help', 'open', 'alert', 'start', 'stop', 'exit'")
                                        .desired_width(f32::INFINITY)
                                        .show(ui);
                                    if r.response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                        let line = std::mem::take(&mut self.consoles[i].input_buf);
                                        let _ = self.consoles[i].io.to_vm.send(line);
                                    }
                                });
                            if close_me {
                                self.consoles.remove(i);
                                break;
                            }
                        }
                    });
            });
        });

        // Simple “About/Alert” popup
        if let Some(msg) = self.alert_text.take() {
            egui::Window::new("Message")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| { ui.label(msg); });
        }
    }
}

// A tiny shim so we can wire egui actions into the StubBasil host callbacks.
// In your real build, use the HostBridge from host_api and pass closures that capture &mut GuiApp state.
struct HostShim {
    open_dialog: Box<dyn Fn() -> Option<String> + Send + Sync>,
    alert_fn:    Box<dyn Fn(&str) + Send + Sync>,
    set_anim_fn: Box<dyn Fn(bool) + Send + Sync>,
}
impl basil_embed::HostApi for HostShim {
    fn open_file(&self) -> Option<String> { (self.open_dialog)() }
    fn start_anim(&self) { (self.set_anim_fn)(true) }
    fn stop_anim(&self)  { (self.set_anim_fn)(false) }
    fn alert(&self, msg: &str) { (self.alert_fn)(msg) }
}
```

> Notes:
>
> * For simplicity, I kept the **alert/anim callbacks** as no-ops in `open_basil_console()`; in a real pass, capture a `Weak` pointer to `GuiApp` state (or send an event through a channel) so Basil calls can **mutate UI state** safely on the UI thread.
> * eframe/egui is single-threaded UI; your Basil VM runs on a background thread. Use channels (like above) to forward events.

---

# How Basil calls host features (the “mod”)

In your real engine adapter (instead of `StubBasil`), **register** something like:

* `APP.OPEN_FILE$()` → `host.open_file()` → return string to Basil
* `APP.START_ANIM%()` → `host.start_anim()`
* `APP.STOP_ANIM%()` → `host.stop_anim()`
* `APP.ALERT%(msg$)` → `host.alert(msg$)`

That’s identical to how your Feature Objects work—just that **this mod is supplied by the parent app** at embedding time, not compiled into Basil itself.

---

# Why this works well

* **Each Basil window is an independent VM** → users can run multiple scripts simultaneously.
* **Clean lifetime** → typing `exit` tears down the runner and closes the console.
* **Extensible host mod** → add more GUI actions as you go (open project, export image, start/stop render, etc.).
* **No commitment to a GUI framework** → swap eframe for Iced/Slint/Tauri later; the embedding & host-API pattern stays the same.

---
