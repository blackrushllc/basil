use std::env;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn basilc_test_mode_basic() {
    // Resolve basilc exe via Cargo's env var
    let exe_path = if let Ok(p) = env::var("CARGO_BIN_EXE_basilc") { std::path::PathBuf::from(p) } else {
        let md = env::var("CARGO_MANIFEST_DIR").unwrap();
        let mut p = std::path::PathBuf::from(md);
        p.pop(); // up to workspace root
        p.push("target"); p.push("debug");
        if cfg!(windows) { p.push("basilc.exe"); } else { p.push("basilc"); }
        p
    };
    if !exe_path.exists() {
        eprintln!("basilc binary not found at {:?}; skipping test", exe_path);
        return;
    }
    let exe = exe_path;

    // Create a temporary Basil source file
    let mut p = env::temp_dir();
    p.push(format!("testmode_{}.basil", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    let src_path = p;
    let program = r#"' Ask for confirmation
PRINT "Are you sure? (Y/N)";
LET A$ = INPUT$("> ");
PRINT A$;
REM After line input
PRINT "Press a key:";
LET C$ = INPUTC$("");
PRINT C$;
LET S$ = INKEY$();
PRINT S$;
LET I% = INKEY%();
PRINT I%;
"#;
    fs::write(&src_path, program).expect("write temp basil file");

    // Run in test mode with fixed seed
    let output = Command::new(&exe)
        .arg("test")
        .arg(&src_path)
        .arg("--seed")
        .arg("123456")
        .arg("--trace")
        .output()
        .expect("run basilc test");

    assert!(output.status.success(), "basilc test failed: {}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Basic assertions: comments echoed and mock input lines printed
    assert!(stdout.contains("COMMENT: Ask for confirmation"), "stdout missing comment echo:\n{}", stdout);
    assert!(stdout.contains("COMMENT: After line input"), "stdout missing second comment echo:\n{}", stdout);
    assert!(stdout.contains("Mock input to INPUT given as"), "stdout missing mock INPUT log:\n{}", stdout);
    assert!(stdout.contains("Mock input to INPUTC$ given as"), "stdout missing mock INPUTC$ log:\n{}", stdout);
    assert!(stdout.contains("Mock input to INKEY$ given as"), "stdout missing mock INKEY$ log:\n{}", stdout);
    assert!(stdout.contains("Mock input to INKEY% given as"), "stdout missing mock INKEY% log:\n{}", stdout);

    // Clean up
    let _ = fs::remove_file(&src_path);
}
