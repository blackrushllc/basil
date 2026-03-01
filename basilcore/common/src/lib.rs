/*

 ▄▄▄▄    ██▓    ▄▄▄       ▄████▄   ██ ▄█▀ ██▀███   █    ██   ██████  ██░ ██
▓█████▄ ▓██▒   ▒████▄    ▒██▀ ▀█   ██▄█▒ ▓██ ▒ ██▒ ██  ▓██▒▒██    ▒ ▓██░ ██▒
▒██▒ ▄██▒██░   ▒██  ▀█▄  ▒▓█    ▄ ▓███▄░ ▓██ ░▄█ ▒▓██  ▒██░░ ▓██▄   ▒██▀▀██░
▒██░█▀  ▒██░   ░██▄▄▄▄██ ▒▓▓▄ ▄██▒▓██ █▄ ▒██▀▀█▄  ▓▓█  ░██░  ▒   ██▒░▓█ ░██
░▓█  ▀█▓░██████▒▓█   ▓██▒▒ ▓███▀ ░▒██▒ █▄░██▓ ▒██▒▒▒█████▓ ▒██████▒▒░▓█▒░██▓
░▒▓███▀▒░ ▒░▓  ░▒▒   ▓▒█░░ ░▒ ▒  ░▒ ▒▒ ▓▒░ ▒▓ ░▒▓░░▒▓▒ ▒ ▒ ▒ ▒▓▒ ▒ ░ ▒ ░░▒░▒
▒░▒   ░ ░ ░ ▒  ░ ▒   ▒▒ ░  ░  ▒   ░ ░▒ ▒░  ░▒ ░ ▒░░░▒░ ░ ░ ░ ░▒  ░ ░ ▒ ░▒░ ░
 ░    ░   ░ ░    ░   ▒   ░        ░ ░░ ░   ░░   ░  ░░░ ░ ░ ░  ░  ░   ░  ░░ ░
 ░          ░  ░     ░  ░░ ░      ░  ░      ░        ░           ░   ░  ░  ░
      ░                  ░
Copyright (C) 2026, Blackrush LLC
Created by Erik Olson, Tarpon Springs, Florida
For more information, visit BlackrushDrive.com

MIT License

Copyright (c) 2026 Erik Lee Olson for Blackrush, LLC

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

*/
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span { pub start: u32, pub end: u32 }
impl Span { pub fn new(start: usize, end: usize) -> Self { Self { start: start as u32, end: end as u32 } } }


#[derive(Debug)]
pub struct BasilError(pub String);
impl std::fmt::Display for BasilError { fn fmt(&self, f:&mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) } }
impl std::error::Error for BasilError {}

pub fn basil_error(line: u32, msg: &str) -> BasilError {
    BasilError(format!("[line {}] {}", line, msg))
}


pub type Result<T> = std::result::Result<T, BasilError>;

use chrono::{DateTime, Local};
use std::path::Path;

pub fn get_file_date(path: Option<&Path>) -> String {
    path.and_then(|p| p.metadata().ok())
        .and_then(|m| m.modified().ok())
        .map(|t| {
            let datetime: DateTime<Local> = t.into();
            datetime.format("%Y-%m-%d %H:%M:%S").to_string()
        })
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn print_suite_version(current_tool: &str, version: &str) {
    let current_exe = std::env::current_exe().ok();

    println!("Basil BASIC Suite - {} v{}", current_tool, version);
    println!("Builds in this suite:");

    // 1. current_tool (assumed to be the same version)
    println!("  - {:<12} {} ({})", format!("{}:", current_tool), version, get_file_date(current_exe.as_deref()));

    // 2. Siblings
    if let Some(dir) = current_exe.as_ref().and_then(|p| p.parent()) {
        let tools = ["basilc", "bcc", "basil-serve"];
        for tool in tools {
            if tool == current_tool { continue; }
            
            let tool_exe = if cfg!(windows) { format!("{}.exe", tool) } else { tool.to_string() };
            let tool_path = dir.join(tool_exe);
            
            if tool_path.exists() {
                println!("  - {:<12} {} ({})", format!("{}:", tool), version, get_file_date(Some(&tool_path)));
            } else {
                println!("  - {:<12} {} (not found in suite directory)", format!("{}:", tool), version);
            }
        }
    }
}