//! Phase 1 preprocessor for Basil.
//! Implements: `#include` with include-once, basic search paths, and embedded/library lookup.

use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum MacroValue {
    Bool(bool),
    Int(i64),
    Str(String),
}

#[derive(Debug, Clone)]
pub enum IncludeKey {
    Fs(PathBuf),
    Embedded(String),
}

pub trait IncludeProvider {
    fn try_read(&self, logical: &str) -> Option<&'static [u8]>;
}

#[derive(Clone)]
pub struct PreprocessOptions<'a> {
    pub root_path: PathBuf,
    pub include_paths: Vec<PathBuf>,
    pub env_paths: Vec<PathBuf>,
    pub embedded: Option<&'a dyn IncludeProvider>,
    pub defines: HashMap<String, MacroValue>,
    // Optional hints for built-ins
    pub engine_name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    // Interned file names; index fits in u16 for compact line entries
    pub files: Vec<String>,
    // 1-based preprocessed line -> (file_idx, line_in_file)
    pub lines: Vec<(u16, u32)>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            lines: vec![(0, 0)],
        }
    } // slot 0 unused so indices are 1-based
    fn intern_file(&mut self, name: &str) -> u16 {
        if let Some(idx) = self.files.iter().position(|s| s == name) {
            return idx as u16;
        }
        self.files.push(name.to_string());
        (self.files.len() - 1) as u16
    }
    fn push_line(&mut self, file_idx: u16, line_in_file: u32) {
        self.lines.push((file_idx, line_in_file));
    }
}

#[derive(Debug, Clone)]
pub struct PreprocessResult {
    pub text: String,
    pub source_map: SourceMap,
    pub dependencies: Vec<IncludeKey>,
}

#[derive(thiserror::Error, Debug)]
pub enum PreprocessError {
    #[error("preprocessor error: {0}")]
    Message(String),
    #[error("include not found: {0}")]
    NotFound(String),
    #[error("include cycle detected: {0}")]
    Cycle(String),
}

/// Preprocess a single source text, expanding `#include` directives.
///
/// Supported forms on a line where `#` is the first non-whitespace char:
///   #include "path/to/file.basil"
///   #include <embedded/path.basil>
///   #include path/without/spaces.basil
pub fn preprocess<'a>(
    text: &str,
    opts: &PreprocessOptions<'a>,
) -> Result<PreprocessResult, PreprocessError> {
    let mut out = String::new();
    let mut deps: Vec<IncludeKey> = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut size_limit: usize = 2 * 1024 * 1024; // 2 MiB
    let max_depth: usize = 64;

    // For the root, the current directory is the directory of root_path if it is a file, else root_path itself
    let root_dir = if opts.root_path.is_file() {
        opts.root_path
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf()
    } else {
        opts.root_path.clone()
    };

    // Macro table starts with provided defines
    let mut macros: HashMap<String, MacroValue> = opts.defines.clone();

    // Active conditional stack; true means content is enabled at this nesting level
    let mut cond_stack: Vec<CondFrame> = Vec::new();

    let current_file = display_root_file(&opts.root_path);
    let mut smap = SourceMap::new();
    expand(
        text,
        &root_dir,
        &current_file,
        opts,
        0,
        max_depth,
        &mut out,
        &mut size_limit,
        &mut visited,
        &mut deps,
        &mut Vec::new(),
        &mut macros,
        &mut cond_stack,
        &mut smap,
    )?;

    Ok(PreprocessResult {
        text: out,
        source_map: smap,
        dependencies: deps,
    })
}

fn expand<'a>(
    text: &str,
    cur_dir: &Path,
    current_file: &str,
    opts: &PreprocessOptions<'a>,
    depth: usize,
    max_depth: usize,
    out: &mut String,
    size_left: &mut usize,
    visited: &mut HashSet<String>,
    deps: &mut Vec<IncludeKey>,
    stack: &mut Vec<String>,
    macros: &mut HashMap<String, MacroValue>,
    cond_stack: &mut Vec<CondFrame>,
    smap: &mut SourceMap,
) -> Result<(), PreprocessError> {
    if depth > max_depth {
        return Err(PreprocessError::Message(format!(
            "maximum include depth ({}) exceeded",
            max_depth
        )));
    }

    for (lineno0, raw_line) in text.lines().enumerate() {
        let lineno = lineno0 + 1;
        // find first non-whitespace
        let mut i = 0usize;
        let line_bytes = raw_line.as_bytes();
        while i < line_bytes.len() && line_bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let directive = if i < line_bytes.len() && line_bytes[i] == b'#' {
            Some(&raw_line[i..])
        } else {
            None
        };
        if let Some(rest) = directive {
            let rest_trim = rest.trim_start_matches('#').trim_start();
            // Handle directives regardless of active state if they are structural (#if/#elif/#else/#endif/#define/#undef)
            if let Some(rem) = rest_trim.strip_prefix("if") {
                let expr = rem.trim();
                let parent_active = cond_stack.last().map(|f| f.active).unwrap_or(true);
                let val = if parent_active {
                    eval_expr(expr, macros, current_file, lineno, opts)?
                } else {
                    false
                };
                cond_stack.push(CondFrame {
                    parent_active,
                    active: parent_active && val,
                    matched: val,
                    saw_else: false,
                });
                continue;
            } else if let Some(rem) = rest_trim.strip_prefix("elif") {
                let expr = rem.trim();
                let Some(frame) = cond_stack.last_mut() else {
                    return Err(PreprocessError::Message(format!(
                        "#elif without matching #if at line {}",
                        lineno
                    )));
                };
                if frame.saw_else {
                    return Err(PreprocessError::Message(format!(
                        "#elif after #else at line {}",
                        lineno
                    )));
                }
                let val = if frame.parent_active && !frame.matched {
                    eval_expr(expr, macros, current_file, lineno, opts)?
                } else {
                    false
                };
                frame.active = frame.parent_active && !frame.matched && val;
                if val {
                    frame.matched = true;
                }
                continue;
            } else if rest_trim == "else" {
                let Some(frame) = cond_stack.last_mut() else {
                    return Err(PreprocessError::Message(format!(
                        "#else without matching #if at line {}",
                        lineno
                    )));
                };
                if frame.saw_else {
                    return Err(PreprocessError::Message(format!(
                        "duplicate #else at line {}",
                        lineno
                    )));
                }
                frame.saw_else = true;
                frame.active = frame.parent_active && !frame.matched;
                continue;
            } else if rest_trim == "endif" {
                if cond_stack.pop().is_none() {
                    return Err(PreprocessError::Message(format!(
                        "#endif without matching #if at line {}",
                        lineno
                    )));
                }
                continue;
            } else if let Some(rem) = rest_trim.strip_prefix("define") {
                let active = cond_stack.iter().all(|f| f.active);
                if active {
                    let def = rem.trim();
                    if def.is_empty() {
                        return Err(PreprocessError::Message(format!(
                            "invalid #define at line {}",
                            lineno
                        )));
                    }
                    // NAME [value]
                    let mut parts = def.splitn(2, char::is_whitespace);
                    let name = parts.next().unwrap().to_string();
                    let val = parts.next().unwrap_or("").trim();
                    if is_builtin(&name) {
                        return Err(PreprocessError::Message(format!(
                            "cannot redefine built-in macro {} at line {}",
                            name, lineno
                        )));
                    }
                    if val.is_empty() {
                        macros.insert(name, MacroValue::Bool(true));
                    } else if let Ok(v) = val.parse::<i64>() {
                        macros.insert(name, MacroValue::Int(v));
                    } else if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
                        macros.insert(name, MacroValue::Str(val[1..val.len() - 1].to_string()));
                    } else {
                        // treat as bare identifier truthy
                        macros.insert(name, MacroValue::Bool(true));
                    }
                }
                continue;
            } else if let Some(rem) = rest_trim.strip_prefix("undef") {
                let active = cond_stack.iter().all(|f| f.active);
                if active {
                    let name = rem.trim();
                    if is_builtin(name) {
                        return Err(PreprocessError::Message(format!(
                            "cannot undef built-in macro {} at line {}",
                            name, lineno
                        )));
                    }
                    macros.remove(name);
                }
                continue;
            } else if rest_trim.starts_with("include") {
                // Only process include if currently active
                let active = cond_stack.iter().all(|f| f.active);
                if !active {
                    continue;
                }
                let after = &rest_trim["include".len()..];
                let arg = after.trim();
                // parse path token: quoted "..." | angle <...> | bare token (no spaces)
                let (form, logical) = parse_include_arg(arg)
                    .map_err(|m| PreprocessError::Message(format!("{} at line {}", m, lineno)))?;
                // resolve and include once
                match form {
                    IncludeForm::Angle => {
                        // Angle: prefer embedded, then search paths
                        if let Some(bytes) = opts.embedded.and_then(|e| e.try_read(&logical)) {
                            let key = format!("embedded:/{}", logical);
                            if stack.iter().any(|k| k == &key) {
                                let mut chain = stack.clone();
                                chain.push(format!("<{}>", logical));
                                return Err(PreprocessError::Cycle(render_stack(&chain)));
                            }
                            if visited.insert(key.clone()) {
                                deps.push(IncludeKey::Embedded(logical.clone()));
                                stack.push(key.clone());
                                let s = std::str::from_utf8(bytes).unwrap_or("");
                                expand(
                                    s,
                                    cur_dir,
                                    &format!("<{}>", logical),
                                    opts,
                                    depth + 1,
                                    max_depth,
                                    out,
                                    size_left,
                                    visited,
                                    deps,
                                    stack,
                                    macros,
                                    cond_stack,
                                    smap,
                                )?;
                                stack.pop();
                            }
                            continue;
                        }
                        if let Some(res) = resolve_any(
                            &logical, cur_dir, opts, /*allow_embedded_fallback=*/ false,
                        ) {
                            include_resolved(
                                res, opts, depth, max_depth, out, size_left, visited, deps, stack,
                                macros, cond_stack, smap,
                            )?;
                        } else {
                            return Err(PreprocessError::NotFound(format!(
                                "{} (angle include)",
                                logical
                            )));
                        }
                        continue;
                    }
                    IncludeForm::Quoted | IncludeForm::Bare => {
                        // Quoted/bare: search FS first, then env/CLI, then embedded as fallback
                        if let Some(res) = resolve_any(
                            &logical, cur_dir, opts, /*allow_embedded_fallback=*/ true,
                        ) {
                            include_resolved(
                                res, opts, depth, max_depth, out, size_left, visited, deps, stack,
                                macros, cond_stack, smap,
                            )?;
                        } else {
                            return Err(PreprocessError::NotFound(logical));
                        }
                        continue;
                    }
                }
            }
        }

        // Normal line: append
        // account for size limit (+1 for newline)
        let active = cond_stack.iter().all(|f| f.active);
        if active {
            let need = raw_line.len() + 1;
            if *size_left < need {
                return Err(PreprocessError::Message(
                    "preprocessed output exceeds size limit".into(),
                ));
            }
            out.push_str(raw_line);
            out.push('\n');
            *size_left -= need;
            // record mapping for this newly appended line
            let file_idx = smap.intern_file(current_file);
            smap.push_line(file_idx, lineno as u32);
        }
    }

    // If we are at the end of a unit and at root depth, check unmatched #if
    if depth == 0 && !cond_stack.is_empty() {
        return Err(PreprocessError::Message(
            "unterminated #if (missing #endif)".into(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IncludeForm {
    Quoted,
    Angle,
    Bare,
}

fn parse_include_arg(arg: &str) -> Result<(IncludeForm, String), String> {
    let arg = arg.trim();
    if arg.is_empty() {
        return Err("missing include path".into());
    }
    let bytes = arg.as_bytes();
    if bytes[0] == b'"' {
        if let Some(end) = arg[1..].find('"') {
            return Ok((IncludeForm::Quoted, arg[1..1 + end].to_string()));
        }
        return Err("unterminated string in #include".into());
    }
    if bytes[0] == b'<' {
        if let Some(end) = arg[1..].find('>') {
            return Ok((IncludeForm::Angle, arg[1..1 + end].to_string()));
        }
        return Err("unterminated angle include in #include".into());
    }
    // bare: stop at first whitespace
    let mut end = arg.len();
    for (i, ch) in arg.char_indices() {
        if ch.is_whitespace() {
            end = i;
            break;
        }
    }
    let token = &arg[..end];
    if token.contains(' ') {
        return Err("spaces in include path require quotes".into());
    }
    Ok((IncludeForm::Bare, token.to_string()))
}

enum Resolved {
    Fs(PathBuf, String),
    Embedded(String, String),
}

fn resolve_in_search_paths(
    logical: &str,
    cur_dir: &Path,
    opts: &PreprocessOptions,
) -> Option<(PathBuf, String)> {
    // Ordered: cur_dir, project root (opts.root_path dir), -I include_paths, env_paths
    let mut candidates: Vec<PathBuf> = Vec::new();
    // Accept both separators; normalize by splitting and joining
    let logical_path = Path::new(logical);

    // 1) current file dir
    candidates.push(cur_dir.join(logical_path));
    // 2) project root
    let proj_root = if opts.root_path.is_file() {
        opts.root_path.parent().unwrap_or(Path::new("."))
    } else {
        opts.root_path.as_path()
    };
    candidates.push(proj_root.join(logical_path));
    // 3) -I
    for dir in &opts.include_paths {
        candidates.push(dir.join(logical_path));
    }
    // 4) env paths
    for dir in &opts.env_paths {
        candidates.push(dir.join(logical_path));
    }

    for cand in candidates {
        if let Ok(abs) = fs::canonicalize(&cand) {
            if abs.is_file() {
                if let Ok(s) = fs::read_to_string(&abs) {
                    return Some((abs, s));
                }
            }
        }
    }
    None
}

fn canonical_key(path: &Path) -> String {
    if cfg!(windows) {
        path.to_string_lossy().to_ascii_lowercase()
    } else {
        path.to_string_lossy().to_string()
    }
}

fn render_stack(stack: &[String]) -> String {
    stack.join(" -> ")
}

fn resolve_any(
    logical: &str,
    cur_dir: &Path,
    opts: &PreprocessOptions,
    allow_embedded_fallback: bool,
) -> Option<Resolved> {
    if let Some((abs, content)) = resolve_in_search_paths(logical, cur_dir, opts) {
        return Some(Resolved::Fs(abs, content));
    }
    if allow_embedded_fallback {
        if let Some(bytes) = opts.embedded.and_then(|e| e.try_read(logical)) {
            let content = String::from_utf8_lossy(bytes).to_string();
            return Some(Resolved::Embedded(logical.to_string(), content));
        }
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn include_resolved<'a>(
    res: Resolved,
    opts: &PreprocessOptions<'a>,
    depth: usize,
    max_depth: usize,
    out: &mut String,
    size_left: &mut usize,
    visited: &mut HashSet<String>,
    deps: &mut Vec<IncludeKey>,
    stack: &mut Vec<String>,
    macros: &mut HashMap<String, MacroValue>,
    cond_stack: &mut Vec<CondFrame>,
    smap: &mut SourceMap,
) -> Result<(), PreprocessError> {
    match res {
        Resolved::Fs(abs, content) => {
            let key = canonical_key(&abs);
            if stack.iter().any(|k| k == &key) {
                let mut chain = stack.clone();
                chain.push(abs.to_string_lossy().to_string());
                return Err(PreprocessError::Cycle(render_stack(&chain)));
            }
            if visited.insert(key.clone()) {
                deps.push(IncludeKey::Fs(abs.clone()));
                stack.push(key.clone());
                let child_file = abs.to_string_lossy().to_string();
                expand(
                    &content,
                    abs.parent().unwrap_or(Path::new(".")).as_ref(),
                    &child_file,
                    opts,
                    depth + 1,
                    max_depth,
                    out,
                    size_left,
                    visited,
                    deps,
                    stack,
                    macros,
                    cond_stack,
                    smap,
                )?;
                stack.pop();
            }
        }
        Resolved::Embedded(logical, content) => {
            let key = format!("embedded:/{}", logical);
            if stack.iter().any(|k| k == &key) {
                let mut chain = stack.clone();
                chain.push(format!("<{}>", logical));
                return Err(PreprocessError::Cycle(render_stack(&chain)));
            }
            if visited.insert(key.clone()) {
                deps.push(IncludeKey::Embedded(logical.clone()));
                stack.push(key.clone());
                expand(
                    &content,
                    Path::new("."),
                    &format!("<{}>", logical),
                    opts,
                    depth + 1,
                    max_depth,
                    out,
                    size_left,
                    visited,
                    deps,
                    stack,
                    macros,
                    cond_stack,
                    smap,
                )?;
                stack.pop();
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct CondFrame {
    parent_active: bool,
    active: bool,
    matched: bool,
    saw_else: bool,
}

fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "__version__" | "__file__" | "__line__" | "__os__" | "__engine__" | "__debug__"
    )
}

#[derive(Clone, Debug)]
enum Val {
    Int(i64),
    Str(String),
    Bool(bool),
}

impl Val {
    fn truthy(&self) -> bool {
        match self {
            Val::Int(i) => *i != 0,
            Val::Str(s) => !s.is_empty(),
            Val::Bool(b) => *b,
        }
    }
}

fn eval_expr(
    expr: &str,
    macros: &HashMap<String, MacroValue>,
    current_file: &str,
    line: usize,
    opts: &PreprocessOptions,
) -> Result<bool, PreprocessError> {
    let mut p = Parser::new(expr);
    let resolver = |name: &str| resolve_ident(name, macros, current_file, line, opts);
    let val = p.parse_expr(&resolver)?;
    Ok(val.truthy())
}

fn resolve_ident(
    name: &str,
    macros: &HashMap<String, MacroValue>,
    current_file: &str,
    line: usize,
    opts: &PreprocessOptions,
) -> Option<Val> {
    if name == "defined" {
        return None;
    } // handled specially as function
    match name {
        "__file__" => Some(Val::Str(current_file.to_string())),
        "__line__" => Some(Val::Int(line as i64)),
        "__engine__" => Some(Val::Str(
            opts.engine_name
                .clone()
                .unwrap_or_else(|| "basilc".to_string()),
        )),
        "__version__" => Some(Val::Str(
            opts.version.clone().unwrap_or_else(|| "0".to_string()),
        )),
        "__debug__" => Some(Val::Int(0)),
        "__os__" => Some(Val::Str(current_os_string())),
        _ => macros.get(name).map(|m| match m {
            MacroValue::Bool(b) => Val::Bool(*b),
            MacroValue::Int(i) => Val::Int(*i),
            MacroValue::Str(s) => Val::Str(s.clone()),
        }),
    }
}

fn current_os_string() -> String {
    if cfg!(target_os = "windows") {
        "windows".into()
    } else if cfg!(target_os = "macos") {
        "macos".into()
    } else {
        "linux".into()
    }
}

struct Parser<'a> {
    s: &'a str,
    i: usize,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self {
        Self { s, i: 0 }
    }
    fn peek(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }
    fn bump(&mut self) -> Option<char> {
        if let Some(c) = self.peek() {
            self.i += c.len_utf8();
            Some(c)
        } else {
            None
        }
    }
    fn eat_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.bump();
            } else {
                break;
            }
        }
    }

    fn parse_expr<F: Fn(&str) -> Option<Val>>(
        &mut self,
        resolve: &F,
    ) -> Result<Val, PreprocessError> {
        self.eat_ws();
        let v = self.parse_or(resolve)?;
        self.eat_ws();
        Ok(v)
    }

    fn parse_or<F: Fn(&str) -> Option<Val>>(
        &mut self,
        resolve: &F,
    ) -> Result<Val, PreprocessError> {
        let mut left = self.parse_and(resolve)?;
        loop {
            self.eat_ws();
            if self.match_str("||") {
                let right = self.parse_and(resolve)?;
                left = Val::Bool(left.truthy() || right.truthy());
            } else {
                break;
            }
        }
        Ok(left)
    }
    fn parse_and<F: Fn(&str) -> Option<Val>>(
        &mut self,
        resolve: &F,
    ) -> Result<Val, PreprocessError> {
        let mut left = self.parse_cmp(resolve)?;
        loop {
            self.eat_ws();
            if self.match_str("&&") {
                let right = self.parse_cmp(resolve)?;
                left = Val::Bool(left.truthy() && right.truthy());
            } else {
                break;
            }
        }
        Ok(left)
    }
    fn parse_cmp<F: Fn(&str) -> Option<Val>>(
        &mut self,
        resolve: &F,
    ) -> Result<Val, PreprocessError> {
        let mut left = self.parse_unary(resolve)?;
        loop {
            self.eat_ws();
            if let Some(op) = self.match_any(&["==", "!=", "<=", ">=", "<", ">"]) {
                let right = self.parse_unary(resolve)?;
                left = Val::Bool(compare(&left, op, &right).ok_or_else(|| {
                    PreprocessError::Message(format!("invalid comparison types for '{}'", op))
                })?);
            } else {
                break;
            }
        }
        Ok(left)
    }
    fn parse_unary<F: Fn(&str) -> Option<Val>>(
        &mut self,
        resolve: &F,
    ) -> Result<Val, PreprocessError> {
        self.eat_ws();
        if self.match_char('!') {
            let v = self.parse_unary(resolve)?;
            return Ok(Val::Bool(!v.truthy()));
        }
        self.parse_primary(resolve)
    }
    fn parse_primary<F: Fn(&str) -> Option<Val>>(
        &mut self,
        resolve: &F,
    ) -> Result<Val, PreprocessError> {
        self.eat_ws();
        if self.match_char('(') {
            let v = self.parse_expr(resolve)?;
            self.eat_ws();
            if !self.match_char(')') {
                return Err(PreprocessError::Message("expected ')'".into()));
            }
            return Ok(v);
        }
        if self.peek() == Some('"') {
            return Ok(Val::Str(self.parse_string()?));
        }
        if let Some(d) = self.peek() {
            if d.is_ascii_digit() || d == '-' {
                return Ok(Val::Int(self.parse_int()?));
            }
        }
        // identifier or defined(NAME)
        let ident = self.parse_ident()?;
        if ident == "defined" {
            self.eat_ws();
            if !self.match_char('(') {
                return Err(PreprocessError::Message(
                    "expected '(' after defined".into(),
                ));
            }
            let name = self.parse_ident()?;
            self.eat_ws();
            if !self.match_char(')') {
                return Err(PreprocessError::Message(
                    "expected ')' after defined(NAME)".into(),
                ));
            }
            return Ok(Val::Bool(resolve(&name).is_some()));
        }
        if let Some(v) = resolve(&ident) {
            Ok(v)
        } else {
            Ok(Val::Bool(false))
        }
    }

    fn parse_ident(&mut self) -> Result<String, PreprocessError> {
        self.eat_ws();
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' || c == '.' {
                s.push(c);
                self.bump();
            } else {
                break;
            }
        }
        if s.is_empty() {
            Err(PreprocessError::Message("expected identifier".into()))
        } else {
            Ok(s)
        }
    }
    fn parse_string(&mut self) -> Result<String, PreprocessError> {
        if !self.match_char('"') {
            return Err(PreprocessError::Message("expected string".into()));
        }
        let mut s = String::new();
        while let Some(c) = self.bump() {
            if c == '"' {
                return Ok(s);
            }
            if c == '\\' {
                if let Some(n) = self.bump() {
                    s.push(n);
                } else {
                    break;
                }
            } else {
                s.push(c);
            }
        }
        Err(PreprocessError::Message(
            "unterminated string literal".into(),
        ))
    }
    fn parse_int(&mut self) -> Result<i64, PreprocessError> {
        let mut s = String::new();
        if self.peek() == Some('-') {
            s.push('-');
            self.bump();
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                s.push(c);
                self.bump();
            } else {
                break;
            }
        }
        s.parse::<i64>()
            .map_err(|_| PreprocessError::Message("invalid integer".into()))
    }
    fn match_char(&mut self, ch: char) -> bool {
        if self.peek() == Some(ch) {
            self.bump();
            true
        } else {
            false
        }
    }
    fn match_str(&mut self, s: &str) -> bool {
        if self.s[self.i..].starts_with(s) {
            self.i += s.len();
            true
        } else {
            false
        }
    }
    fn match_any<'b>(&mut self, ops: &'b [&str]) -> Option<&'b str> {
        for &op in ops {
            if self.s[self.i..].starts_with(op) {
                self.i += op.len();
                return Some(op);
            }
        }
        None
    }
}

fn compare(a: &Val, op: &str, b: &Val) -> Option<bool> {
    match (a, b) {
        (Val::Int(x), Val::Int(y)) => Some(match op {
            "==" => x == y,
            "!=" => x != y,
            "<" => x < y,
            ">" => x > y,
            "<=" => x <= y,
            ">=" => x >= y,
            _ => return None,
        }),
        (Val::Str(x), Val::Str(y)) => Some(match op {
            "==" => x == y,
            "!=" => x != y,
            "<" => x < y,
            ">" => x > y,
            "<=" => x <= y,
            ">=" => x >= y,
            _ => return None,
        }),
        (Val::Bool(x), Val::Bool(y)) => Some(match op {
            "==" => x == y,
            "!=" => x != y,
            _ => return None,
        }),
        _ => None,
    }
}

fn display_root_file(root_path: &Path) -> String {
    if root_path.is_file() {
        root_path.to_string_lossy().to_string()
    } else {
        root_path.to_string_lossy().to_string()
    }
}
