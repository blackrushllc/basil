// Terminal screen manipulation utilities for Basil, powered by Crossterm.
// Exposed via VM builtins when feature "obj-term" is enabled.

#![allow(dead_code)]

use std::sync::{Mutex, OnceLock};
use std::io::{self, Write};
use std::time::Duration;

use basil_bytecode::Value;
use crossterm::{execute, cursor, style::{Color, SetForegroundColor, SetBackgroundColor, ResetColor, SetAttribute, Attribute}, terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen}};
use crossterm::event::{self, Event, KeyEvent, KeyCode};
use crossterm::tty::IsTty;

#[derive(Debug, Default, Clone)]
struct TermState {
    fg: Option<Color>,
    bg: Option<Color>,
    bold: bool,
    underline: bool,
    reverse: bool,
    pos_stack: Vec<(u16,u16)>,
    last_err: Option<String>,
    // Phase 2 additions
    initialized: bool,
    raw_on: bool,
    alt_on: bool,
}

static GLOBAL: OnceLock<Mutex<TermState>> = OnceLock::new();

fn state() -> &'static Mutex<TermState> { GLOBAL.get_or_init(|| Mutex::new(TermState::default())) }

fn set_err(msg: String) { let mut s = state().lock().unwrap(); s.last_err = Some(msg); }

fn ok() -> i64 { 0 }

// Public entry point called by VM on init (no-op for now; ensures state exists)
pub fn register(_reg: &mut crate::Registry) { let _ = state(); }

pub fn term_err() -> String {
    let mut s = state().lock().unwrap();
    let out = s.last_err.clone().unwrap_or_default();
    s.last_err = None;
    out
}

fn color_from_code(code: i64) -> Option<Color> {
    match code {
        0 => Some(Color::Black),
        1 => Some(Color::DarkRed),
        2 => Some(Color::DarkGreen),
        3 => Some(Color::DarkYellow),
        4 => Some(Color::DarkBlue),
        5 => Some(Color::DarkMagenta),
        6 => Some(Color::DarkCyan),
        7 => Some(Color::White),
        8 => Some(Color::Grey),
        9 => Some(Color::Red),
        10 => Some(Color::Green),
        11 => Some(Color::Yellow),
        12 => Some(Color::Blue),
        13 => Some(Color::Magenta),
        14 => Some(Color::Cyan),
        15 => Some(Color::White), // Crossterm lacks BrightWhite distinct from White; White is bright
        _ => None,
    }
}

fn color_from_name(name: &str) -> Option<Color> {
    match &*name.to_ascii_lowercase() {
        "black" => Some(Color::Black),
        "red" => Some(Color::DarkRed),
        "green" => Some(Color::DarkGreen),
        "yellow" => Some(Color::DarkYellow),
        "blue" => Some(Color::DarkBlue),
        "magenta" => Some(Color::DarkMagenta),
        "cyan" => Some(Color::DarkCyan),
        "white" => Some(Color::White),
        "grey" | "gray" => Some(Color::Grey),
        "brightred" => Some(Color::Red),
        "brightgreen" => Some(Color::Green),
        "brightyellow" => Some(Color::Yellow),
        "brightblue" => Some(Color::Blue),
        "brightmagenta" => Some(Color::Magenta),
        "brightcyan" => Some(Color::Cyan),
        "brightwhite" => Some(Color::White),
        _ => None
    }
}

fn parse_color_value(v: &Value) -> std::result::Result<Option<Color>, String> {
    match v {
        Value::Int(i) => {
            if *i == -1 { Ok(None) } else { color_from_code(*i).ok_or_else(|| format!("invalid color code {}", i)).map(Some) }
        }
        Value::Num(n) => {
            let i = n.trunc() as i64; if i == -1 { Ok(None) } else { color_from_code(i).ok_or_else(|| format!("invalid color code {}", i)).map(Some) }
        }
        Value::Str(s) => {
            if s == "-1" { Ok(None) } else { color_from_name(s).ok_or_else(|| format!("invalid color name '{}'", s)).map(Some) }
        }
        other => Err(format!("color expects int 0..15, -1, or name; got {}", other)),
    }
}

fn to_i64(v: &Value) -> std::result::Result<i64, String> {
    match v { Value::Int(i)=>Ok(*i), Value::Num(n)=>Ok(n.trunc() as i64), other=>Err(format!("expected integer, got {}", other)) }
}

pub fn cls() -> i64 {
    let s = state().lock().unwrap();
    // Apply current colors so the clear uses them
    let mut out = io::stdout();
    if let Some(fg) = s.fg { let _ = execute!(out, SetForegroundColor(fg)); }
    if let Some(bg) = s.bg { let _ = execute!(out, SetBackgroundColor(bg)); }
    match execute!(out, Clear(ClearType::All), cursor::MoveTo(0,0)) {
        Ok(_) => ok(),
        Err(e) => { set_err(format!("CLS failed: {}", e)); 1 }
    }
}

pub fn locate(x: &Value, y: &Value) -> i64 {
    let xi = match to_i64(x) { Ok(v)=>v, Err(m)=>{ set_err(m); return 2; } };
    let yi = match to_i64(y) { Ok(v)=>v, Err(m)=>{ set_err(m); return 2; } };
    let (cols, rows) = match terminal::size() { Ok(sz)=>sz, Err(e)=>{ set_err(format!("terminal size: {}", e)); (80,25) } };
    let mut xc = if xi < 1 { 1 } else { xi as u16 };
    let mut yc = if yi < 1 { 1 } else { yi as u16 };
    if xc > cols { xc = cols; }
    if yc > rows { yc = rows; }
    match execute!(io::stdout(), cursor::MoveTo(xc-1, yc-1)) { Ok(_)=>ok(), Err(e)=>{ set_err(format!("LOCATE failed: {}", e)); 1 } }
}

pub fn color(fg: &Value, bg: &Value) -> i64 {
    let mut st = state().lock().unwrap();
    let fgc = match parse_color_value(fg) { Ok(c)=>c, Err(m)=>{ set_err(m); return 3; } };
    let bgc = match parse_color_value(bg) { Ok(c)=>c, Err(m)=>{ set_err(m); return 3; } };
    let mut out = io::stdout();
    if let Some(c) = fgc { st.fg = Some(c); if let Err(e) = execute!(out, SetForegroundColor(c)) { set_err(format!("set fg: {}", e)); return 1; } }
    if let Some(c) = bgc { st.bg = Some(c); if let Err(e) = execute!(out, SetBackgroundColor(c)) { set_err(format!("set bg: {}", e)); return 1; } }
    ok()
}

pub fn color_reset() -> i64 {
    let mut st = state().lock().unwrap();
    st.fg = None; st.bg = None;
    if let Err(e) = execute!(io::stdout(), ResetColor) { set_err(format!("ResetColor: {}", e)); return 1; }
    ok()
}

pub fn attr(bold: &Value, underline: &Value, reverse: &Value) -> i64 {
    let bi = match to_i64(bold) { Ok(v)=>v, Err(m)=>{ set_err(m); return 2; } };
    let ui = match to_i64(underline) { Ok(v)=>v, Err(m)=>{ set_err(m); return 2; } };
    let ri = match to_i64(reverse) { Ok(v)=>v, Err(m)=>{ set_err(m); return 2; } };
    let mut st = state().lock().unwrap();
    let mut out = io::stdout();
    if bi == 0 { let _ = execute!(out, SetAttribute(Attribute::NoBold)); st.bold = false; }
    else if bi == 1 { let _ = execute!(out, SetAttribute(Attribute::Bold)); st.bold = true; }
    if ui == 0 { let _ = execute!(out, SetAttribute(Attribute::NoUnderline)); st.underline = false; }
    else if ui == 1 { let _ = execute!(out, SetAttribute(Attribute::Underlined)); st.underline = true; }
    if ri == 0 { let _ = execute!(out, SetAttribute(Attribute::NoReverse)); st.reverse = false; }
    else if ri == 1 { let _ = execute!(out, SetAttribute(Attribute::Reverse)); st.reverse = true; }
    ok()
}

pub fn attr_reset() -> i64 {
    let mut st = state().lock().unwrap();
    st.bold = false; st.underline = false; st.reverse = false;
    if let Err(e) = execute!(io::stdout(), SetAttribute(Attribute::Reset)) { set_err(format!("ATTR_RESET: {}", e)); return 1; }
    ok()
}

pub fn cursor_save() -> i64 {
    match cursor::position() {
        Ok((x,y)) => {
            let mut st = state().lock().unwrap();
            if st.pos_stack.len() >= 8 { st.pos_stack.remove(0); }
            st.pos_stack.push((x,y));
            ok()
        }
        Err(e) => { set_err(format!("CURSOR_SAVE: {}", e)); 1 }
    }
}

pub fn cursor_restore() -> i64 {
    let mut st = state().lock().unwrap();
    if let Some((x,y)) = st.pos_stack.pop() {
        match execute!(io::stdout(), cursor::MoveTo(x, y)) { Ok(_)=>ok(), Err(e)=>{ set_err(format!("CURSOR_RESTORE: {}", e)); 1 } }
    } else { ok() }
}

pub fn term_cols() -> i64 { terminal::size().map(|(c,_)| c as i64).unwrap_or(80) }
pub fn term_rows() -> i64 { terminal::size().map(|(_,r)| r as i64).unwrap_or(25) }

pub fn cursor_hide() -> i64 { match execute!(io::stdout(), cursor::Hide) { Ok(_)=>ok(), Err(e)=>{ set_err(format!("CURSOR_HIDE: {}", e)); 1 } } }
pub fn cursor_show() -> i64 { match execute!(io::stdout(), cursor::Show) { Ok(_)=>ok(), Err(e)=>{ set_err(format!("CURSOR_SHOW: {}", e)); 1 } } }

// -------- Phase 2 additions --------
fn stdout_is_tty() -> bool { io::stdout().is_tty() }

pub fn term_init() -> i64 {
    let mut st = state().lock().unwrap();
    if st.initialized { return ok(); }
    st.initialized = true;
    ok()
}

pub fn term_end() -> i64 {
    // Always try to restore sane state; ignore errors
    let mut st = state().lock().unwrap();
    let mut out = io::stdout();
    let _ = execute!(out, cursor::Show);
    let _ = execute!(out, SetAttribute(Attribute::Reset));
    let _ = execute!(out, ResetColor);
    if st.raw_on { let _ = terminal::disable_raw_mode(); st.raw_on = false; }
    if st.alt_on { let _ = execute!(out, LeaveAlternateScreen); st.alt_on = false; }
    st.initialized = false;
    ok()
}

pub fn term_raw(mode: &Value) -> i64 {
    // Accept: 0/1, or strings "ON"/"OFF" (case-insensitive)
    let on = match mode {
        Value::Int(i) => *i != 0,
        Value::Num(n) => n.trunc() as i64 != 0,
        Value::Str(s) => {
            let up = s.to_ascii_uppercase();
            up == "ON" || up == "1" || up == "TRUE"
        }
        _ => { set_err(format!("TERM.RAW expects ON/OFF or 0/1, got {}", mode)); return 2; }
    };
    if !stdout_is_tty() { return ok(); }
    let mut st = state().lock().unwrap();
    if on {
        if !st.raw_on {
            if let Err(e) = terminal::enable_raw_mode() { set_err(format!("enable_raw_mode: {}", e)); return 1; }
            st.raw_on = true;
        }
    } else {
        if st.raw_on {
            if let Err(e) = terminal::disable_raw_mode() { set_err(format!("disable_raw_mode: {}", e)); return 1; }
            st.raw_on = false;
        }
    }
    ok()
}

pub fn altscreen_on() -> i64 {
    if !stdout_is_tty() { return ok(); }
    let mut st = state().lock().unwrap();
    if !st.alt_on {
        if let Err(e) = execute!(io::stdout(), EnterAlternateScreen) { set_err(format!("ALTSCREEN_ON: {}", e)); return 1; }
        st.alt_on = true;
    }
    ok()
}

pub fn altscreen_off() -> i64 {
    if !stdout_is_tty() { return ok(); }
    let mut st = state().lock().unwrap();
    if st.alt_on {
        if let Err(e) = execute!(io::stdout(), LeaveAlternateScreen) { set_err(format!("ALTSCREEN_OFF: {}", e)); return 1; }
        st.alt_on = false;
    }
    ok()
}

pub fn term_flush() -> i64 {
    match io::stdout().flush() { Ok(_)=>ok(), Err(e)=>{ set_err(format!("FLUSH: {}", e)); 1 } }
}

pub fn term_pollkey_s() -> String {
    if !stdout_is_tty() { return String::new(); }
    let _ = event::EnableMouseCapture; // not used yet, placeholder for future
    // Short non-blocking poll
    match event::poll(Duration::from_millis(0)) {
        Ok(false) => String::new(),
        Ok(true) => {
            match event::read() {
                Ok(Event::Key(KeyEvent { code, .. })) => match code {
                    KeyCode::Enter => "Enter".into(),
                    KeyCode::Esc => "Esc".into(),
                    KeyCode::Tab => "Tab".into(),
                    KeyCode::Backspace => "Backspace".into(),
                    KeyCode::Up => "Up".into(),
                    KeyCode::Down => "Down".into(),
                    KeyCode::Left => "Left".into(),
                    KeyCode::Right => "Right".into(),
                    KeyCode::Char(c) => {
                        let mut s = String::from("Char:"); s.push(c); s
                    }
                    _ => String::new(),
                },
                _ => String::new(),
            }
        }
        Err(_) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_color_name_parser() {
        assert!(color_from_name("brightyellow").is_some());
        assert!(color_from_name("no-such").is_none());
        assert!(parse_color_value(&Value::Int(10)).is_ok());
        assert!(parse_color_value(&Value::Int(99)).is_err());
        assert!(parse_color_value(&Value::Str("blue".into())).is_ok());
        assert!(parse_color_value(&Value::Str("wat".into())).is_err());
    }
    #[test]
    fn test_cursor_stack_save_restore_underflow_ok() {
        let _ = cursor_save();
        let _ = cursor_restore();
        let _ = cursor_restore(); // underflow no-op
    }
    #[test]
    fn test_state_lifecycle_init_end() {
        let _ = term_init();
        let st = state().lock().unwrap().clone();
        assert!(st.initialized);
        let _ = term_end();
        let st2 = state().lock().unwrap().clone();
        assert!(!st2.initialized);
        assert!(!st2.raw_on);
        assert!(!st2.alt_on);
    }

    // Integration-like tests for TTY behavior (ignored by default)
    #[test]
    #[ignore]
    fn test_tty_raw_and_alt_and_poll() {
        if !stdout_is_tty() { return; }
        assert_eq!(term_raw(&Value::Int(1)), 0);
        assert_eq!(term_raw(&Value::Int(0)), 0);
        assert_eq!(altscreen_on(), 0);
        assert_eq!(altscreen_off(), 0);
        let s = term_pollkey_s();
        // Likely no key pressed in short window
        assert!(s == "" || s.starts_with("Char:") || s == "Enter" || s == "Esc" || s == "Tab" || s == "Backspace" || s == "Up" || s == "Down" || s == "Left" || s == "Right");
    }
}
