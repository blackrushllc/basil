#![cfg(feature = "embed-test")]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn basilc_path() -> PathBuf {
    if let Ok(p) = std::env::var("CARGO_BIN_EXE_basilc") {
        return PathBuf::from(p);
    }
    let md = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let mut p = PathBuf::from(md);
    p.pop();
    p.push("target"); p.push("debug");
    if cfg!(windows) { p.push("basilc.exe"); } else { p.push("basilc"); }
    p
}

#[test]
fn list_inventory() {
    let exe = basilc_path();
    if !exe.exists() { eprintln!("no basilc found at {:?}", exe); return; }
    let out = Command::new(&exe)
        .arg("make").arg("--list")
        .env("BASIL_FORCE_MODE", "cli")
        .output().expect("run basilc make --list");
    assert!(out.status.success(), "status: {}\nstdout:\n{}\nstderr:\n{}",
        out.status, String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Embedded files:"));
}

#[test]
fn extract_directory_examples() {
    let exe = basilc_path(); if !exe.exists() { return; }
    let td = tempfile::tempdir().unwrap();
    let out = Command::new(&exe)
        .current_dir(td.path())
        .arg("make").arg("examples")
        .env("BASIL_FORCE_MODE", "cli")
        .output().expect("run basilc make examples");
    assert!(out.status.success(), "{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    // At least one known file should be present if repo includes it
    let p = td.path().join("examples").join("hello.basil");
    assert!(p.exists(), "expected examples/hello.basil to be extracted");
}

#[test]
fn write_single_file_then_skip_run() {
    let exe = basilc_path(); if !exe.exists() { return; }
    let td = tempfile::tempdir().unwrap();
    let out = Command::new(&exe)
        .current_dir(td.path())
        .arg("make").arg("upgrade")
        .env("BASILC_MAKE_SKIP_RUN", "1")
        .env("BASIL_FORCE_MODE", "cli")
        .output().expect("run basilc make upgrade");
    assert!(out.status.success(), "{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let p = td.path().join("upgrade.basil");
    assert!(p.exists(), "expected upgrade.basil to be written");
}

#[test]
fn reject_unsafe_targets() {
    let exe = basilc_path(); if !exe.exists() { return; }
    let td = tempfile::tempdir().unwrap();
    let out = Command::new(&exe)
        .current_dir(td.path())
        .arg("make").arg("../secrets")
        .env("BASIL_FORCE_MODE", "cli")
        .output().expect("run basilc make ../secrets");
    assert!(!out.status.success());
}
