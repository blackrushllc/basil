use egui::{Context, Ui};

use crate::config::{self, BasilicaConfig, MenuItem, MenuKind, MenuMode};
use crate::instance::ConsoleInstance;

pub struct BasilicaApp {
    pub config: BasilicaConfig,
    pub consoles: Vec<ConsoleInstance>,
    pub click_count: i64,
    pub anim_running: bool,
    pub alerts: Vec<String>,
    run_dialog: Option<RunDialogState>,
    manage_open: bool,
}

struct RunDialogState {
    path: Option<String>,
    is_gui: bool,
    mode: MenuMode,
}

impl Default for BasilicaApp {
    fn default() -> Self {
        Self {
            config: config::seed_config(),
            consoles: Vec::new(),
            click_count: 0,
            anim_running: false,
            alerts: Vec::new(),
            run_dialog: None,
            manage_open: false,
        }
    }
}

impl BasilicaApp {
    pub fn new(cfg: BasilicaConfig) -> Self { Self { config: cfg, ..Default::default() } }

    pub fn ui(&mut self, ctx: &Context) {
        // Top menu
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("CLI Scripts", |ui| {
                    for item in self.config.cli_scripts.clone() {
                        if ui.button(item.name.clone()).clicked() { self.launch_item(item, false); ui.close_menu(); }
                    }
                });
                ui.menu_button("GUI Scripts", |ui| {
                    for item in self.config.gui_scripts.clone() {
                        if ui.button(item.name.clone()).clicked() { self.launch_item(item, true); ui.close_menu(); }
                    }
                });
                if ui.button("Run Script…").clicked() { self.run_dialog = Some(RunDialogState{ path: None, is_gui: false, mode: MenuMode::Run }); }
                if ui.button("Manage Scripts…").clicked() { self.manage_open = true; }
                if ui.button("Quit").clicked() { ctx.send_viewport_cmd(egui::ViewportCommand::Close); }
            });
        });

        // Left busy box
        egui::SidePanel::left("busy").show(ctx, |ui| {
            ui.heading("Busy Box");
            if ui.button("Click me").clicked() { self.click_count += 1; }
            ui.label(format!("Count: {}", self.click_count));
            if !self.anim_running { if ui.button("Start animation").clicked() { self.anim_running = true; } }
            else { if ui.button("Stop animation").clicked() { self.anim_running = false; } }
            if ui.button("Open File…").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() { self.alerts.push(format!("Picked {}", path.display())); }
            }
            if !self.alerts.is_empty() { ui.separator(); ui.label("Alerts:"); for a in &self.alerts { ui.label(a); } }
        });

        // Central consoles
        egui::CentralPanel::default().show(ctx, |ui| {
            // Update instances first
            for c in &mut self.consoles { c.update(); }
            // Show each console in its own collapsing header
            for (i, c) in self.consoles.iter_mut().enumerate() {
                egui::CollapsingHeader::new(format!("{}", c.title)).default_open(true).show(ui, |ui| {
                    egui::ScrollArea::vertical().id_salt(format!("log-{}", i)).show(ui, |ui| {
                        ui.monospace(&c.output);
                    });
                    ui.horizontal(|ui| {
                        let resp = ui.text_edit_singleline(&mut c.input);
                        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            let line = std::mem::take(&mut c.input);
                            c.send_line(line + "\n");
                        }
                        if ui.button("Send").clicked() { let line = std::mem::take(&mut c.input); c.send_line(line + "\n"); }
                    });
                });
            }
        });

        // Run Script dialog
        if let Some(state) = &mut self.run_dialog {
            let mut to_launch: Option<(MenuItem, bool)> = None;
            let mut close_now = false;
            egui::Window::new("Run Script…").open(&mut true).show(ctx, |ui| {
                if ui.button("Choose file…").clicked() { if let Some(p) = rfd::FileDialog::new().add_filter("Basil", &["basil"]).pick_file() { state.path = Some(p.to_string_lossy().to_string()); } }
                if let Some(p) = &state.path { ui.label(p); }
                ui.horizontal(|ui| {
                    ui.label("Mode:");
                    ui.selectable_value(&mut state.mode, MenuMode::Run, "run");
                    ui.selectable_value(&mut state.mode, MenuMode::Test, "test");
                    ui.selectable_value(&mut state.mode, MenuMode::Cli, "cli");
                });
                ui.horizontal(|ui| {
                    ui.label("Window:");
                    ui.selectable_value(&mut state.is_gui, false, "CLI window");
                    ui.selectable_value(&mut state.is_gui, true, "GUI window");
                });
                if ui.button("Launch").clicked() {
                    if let Some(p) = &state.path {
                        let item = MenuItem { id: "adhoc".into(), name: format!("Run {}", p), mode: state.mode.clone(), kind: MenuKind::File, path: Some(p.clone()), args: None };
                        to_launch = Some((item, state.is_gui));
                    }
                }
                if ui.button("Close").clicked() { close_now = true; }
            });
            if let Some((item, is_gui)) = to_launch { self.launch_item(item, is_gui); self.run_dialog = None; }
            else if close_now { self.run_dialog = None; }
        }

        if self.manage_open { self.manage_scripts(ctx); }
    }

    fn launch_item(&mut self, item: MenuItem, gui: bool) {
        let title = item.name.clone();
        let initial = Some((item.kind.clone(), item.mode.clone(), item.path.clone()));
        let inst = ConsoleInstance::new(title, gui, initial);
        self.consoles.push(inst);
    }

    fn manage_scripts(&mut self, ctx: &Context) {
        egui::Window::new("Manage Scripts").open(&mut self.manage_open).show(ctx, |ui| {
            egui::widgets::global_theme_preference_buttons(ui);
            ui.horizontal(|ui| {
                if ui.button("Add CLI").clicked() { self.config.cli_scripts.push(blank_item()); }
                if ui.button("Add GUI").clicked() { self.config.gui_scripts.push(blank_item()); }
                if ui.button("Save").clicked() { let _ = config::save_atomic(&self.config); }
            });
            ui.separator();
            ui.label("CLI Scripts");
            render_items_table(ui, &mut self.config.cli_scripts);
            ui.separator();
            ui.label("GUI Scripts");
            render_items_table(ui, &mut self.config.gui_scripts);
        });
    }
}

fn blank_item() -> MenuItem { MenuItem { id: String::new(), name: String::from("New Item"), mode: MenuMode::Cli, kind: MenuKind::Bare, path: None, args: None } }

fn render_items_table(ui: &mut Ui, items: &mut Vec<MenuItem>) {
    let mut to_remove: Option<usize> = None;
    egui::Grid::new("items").striped(true).show(ui, |ui| {
        ui.label("Name"); ui.label("Kind"); ui.label("Path"); ui.label("Mode"); ui.label("Args"); ui.label(""); ui.end_row();
        for (i, it) in items.iter_mut().enumerate() {
            ui.text_edit_singleline(&mut it.name);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut it.kind, MenuKind::Bare, "Bare");
                ui.selectable_value(&mut it.kind, MenuKind::File, "File");
            });
            if it.kind == MenuKind::File {
                let mut p = it.path.clone().unwrap_or_default();
                if ui.text_edit_singleline(&mut p).changed() { it.path = if p.is_empty() { None } else { Some(p) }; }
                if ui.button("…").clicked() { if let Some(sel) = rfd::FileDialog::new().add_filter("Basil", &["basil"]).pick_file() { it.path = Some(sel.to_string_lossy().to_string()); } }
            } else { ui.label(""); }
            ui.horizontal(|ui| {
                ui.selectable_value(&mut it.mode, MenuMode::Run, "run");
                ui.selectable_value(&mut it.mode, MenuMode::Test, "test");
                ui.selectable_value(&mut it.mode, MenuMode::Cli, "cli");
            });
            let mut a = it.args.clone().unwrap_or_default(); if ui.text_edit_singleline(&mut a).changed() { it.args = if a.is_empty() { None } else { Some(a) }; }
            if ui.button("Delete").clicked() { to_remove = Some(i); }
            ui.end_row();
        }
    });
    if let Some(i) = to_remove { items.remove(i); }
}
