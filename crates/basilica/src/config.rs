use std::fs;
use std::io::Write;
use std::path::{PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MenuMode { Run, Test, Cli }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MenuKind { Bare, File }

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MenuItem {
    pub id: String,
    pub name: String,
    pub mode: MenuMode,
    pub kind: MenuKind,
    pub path: Option<String>,
    pub args: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BasilicaConfig {
    pub version: u32,
    pub cli_scripts: Vec<MenuItem>,
    pub gui_scripts: Vec<MenuItem>,
}

pub fn config_dir() -> PathBuf {
    if let Some(pd) = ProjectDirs::from("com", "Basilica", "Basilica") {
        let p = pd.config_dir().to_path_buf();
        // Ensure directory exists
        let _ = fs::create_dir_all(&p);
        p
    } else {
        // Fallback: executable directory
        std::env::current_exe().ok().and_then(|p| p.parent().map(|q| q.to_path_buf())).unwrap_or_else(|| PathBuf::from("."))
    }
}

pub fn config_path() -> PathBuf {
    let mut p = config_dir();
    p.push("basilica.json");
    p
}

pub fn load_or_seed() -> Result<BasilicaConfig> {
    let path = config_path();
    if path.exists() {
        let s = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let cfg: BasilicaConfig = serde_json::from_str(&s).with_context(|| "parse basilica.json")?;
        Ok(cfg)
    } else {
        let cfg = seed_config();
        save_atomic(&cfg)?;
        Ok(cfg)
    }
}

pub fn save_atomic(cfg: &BasilicaConfig) -> Result<()> {
    let path = config_path();
    let tmp = path.with_extension("json.tmp");
    let body = serde_json::to_string_pretty(cfg)?;
    let mut f = fs::File::create(&tmp).with_context(|| format!("create {}", tmp.display()))?;
    f.write_all(body.as_bytes())?;
    f.sync_all()?;
    fs::rename(&tmp, &path).with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

pub fn seed_config() -> BasilicaConfig {
    use MenuKind as K; use MenuMode as M;
    BasilicaConfig {
        version: 1,
        cli_scripts: vec![
            MenuItem{ id: "basil-prompt".into(), name: "Basil Prompt".into(), mode: M::Cli, kind: K::Bare, path: None, args: None },
            MenuItem{ id: "run-hello".into(), name: "Run Hello".into(), mode: M::Run, kind: K::File, path: Some("examples/hello.basil".into()), args: None },
        ],
        gui_scripts: vec![
            MenuItem{ id: "blank-gui-prompt".into(), name: "Blank GUI Prompt".into(), mode: M::Cli, kind: K::Bare, path: None, args: None },
            MenuItem{ id: "gui-hello".into(), name: "GUI Hello".into(), mode: M::Run, kind: K::File, path: Some("examples/gui_hello.basil".into()), args: None },
        ],
    }
}

#[allow(dead_code)]
pub fn to_host_pending(cfg: &BasilicaConfig) -> basil_host::PendingConfig {
    basil_host::PendingConfig {
        cli_scripts: cfg.cli_scripts.iter().map(|m| basil_host::MenuItem{ id: m.id.clone(), name: m.name.clone(), mode: to_mode_str(&m.mode), kind: to_kind_str(&m.kind), path: m.path.clone(), args: m.args.clone() }).collect(),
        gui_scripts: cfg.gui_scripts.iter().map(|m| basil_host::MenuItem{ id: m.id.clone(), name: m.name.clone(), mode: to_mode_str(&m.mode), kind: to_kind_str(&m.kind), path: m.path.clone(), args: m.args.clone() }).collect(),
        saved: false,
    }
}

#[allow(dead_code)]
pub fn from_host_pending(p: &basil_host::PendingConfig) -> BasilicaConfig {
    BasilicaConfig { version: 1,
        cli_scripts: p.cli_scripts.iter().map(|m| MenuItem{ id: m.id.clone(), name: m.name.clone(), mode: from_mode_str(&m.mode), kind: from_kind_str(&m.kind), path: m.path.clone(), args: m.args.clone() }).collect(),
        gui_scripts: p.gui_scripts.iter().map(|m| MenuItem{ id: m.id.clone(), name: m.name.clone(), mode: from_mode_str(&m.mode), kind: from_kind_str(&m.kind), path: m.path.clone(), args: m.args.clone() }).collect() }
}

fn to_mode_str(m: &MenuMode) -> String { match m { MenuMode::Run=>"run".into(), MenuMode::Test=>"test".into(), MenuMode::Cli=>"cli".into() } }
fn to_kind_str(k: &MenuKind) -> String { match k { MenuKind::Bare=>"bare".into(), MenuKind::File=>"file".into() } }
fn from_mode_str(s: &str) -> MenuMode { match s { s if s.eq_ignore_ascii_case("run")=>MenuMode::Run, s if s.eq_ignore_ascii_case("test")=>MenuMode::Test, _=>MenuMode::Cli } }
fn from_kind_str(s: &str) -> MenuKind { match s { s if s.eq_ignore_ascii_case("file")=>MenuKind::File, _=>MenuKind::Bare } }
