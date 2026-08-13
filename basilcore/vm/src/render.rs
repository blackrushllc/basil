use super::VM;
use basil_bytecode::Value;
use basil_common::{BasilError, Result};
use std::cell::RefCell;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

// Public entry point
// Optional context dictionary provides additional variables visible during rendering.
pub fn render_template(
    vm: &mut VM,
    template: &str,
    ctx_dict: Option<Rc<RefCell<HashMap<String, Value>>>>,
) -> Result<String> {
    // Start overlay as a copy of provided context dictionary (if any) so that template-loop variables
    // do not mutate the caller's dictionary values.
    let overlay = if let Some(rc) = ctx_dict {
        let cloned: HashMap<String, Value> = rc
            .borrow()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Some(Rc::new(RefCell::new(cloned)))
    } else {
        None
    };
    let mut ctx = Ctx {
        max_depth: 16,
        depth: 0,
        overlay,
        template: template.to_string(),
    };
    render_inner(vm, template, &mut ctx)
}

struct Ctx {
    max_depth: usize,
    depth: usize,
    overlay: Option<Rc<RefCell<HashMap<String, Value>>>>,
    // Keep a copy of the template to compute line/column on errors
    template: String,
}

#[derive(Debug, Clone)]
enum Node {
    Text(String),
    Interp {
        expr: String,
        pos: usize,
    }, // {{ expr }} starting at byte offset pos
    Call {
        name: String,
        args: Vec<String>,
        pos: usize,
    }, // @NAME(args) at pos
    If {
        cond: String,
        then_part: Vec<Node>,
        else_part: Vec<Node>,
        pos: usize,
    },
    Case {
        arms: Vec<(String, Vec<Node>)>,
        pos: usize,
    },
    // Loops
    ForEach {
        var: String,
        enumerable: String,
        body: Vec<Node>,
        else_part: Vec<Node>,
        order: ForEachOrder,
        pos: usize,
    },
    Times {
        count: String,
        body: Vec<Node>,
        pos: usize,
    },
    ForNum {
        var: String,
        start: String,
        end: String,
        step: Option<String>,
        body: Vec<Node>,
        pos: usize,
    },
    While {
        cond: String,
        body: Vec<Node>,
        pos: usize,
    },
    Break {
        pos: usize,
    },
    Continue {
        pos: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ForEachOrder {
    Natural,
    KSortAsc,
    VSortAsc,
    KSortDesc,
    VSortDesc,
}

fn render_inner(vm: &mut VM, template: &str, ctx: &mut Ctx) -> Result<String> {
    let nodes = parse_template(template)?;
    eval_nodes(vm, &nodes, ctx)
}

fn parse_template(s: &str) -> Result<Vec<Node>> {
    let mut i = 0usize;
    parse_block(s, &mut i, &[])
}

// Parse until one of the end tokens is seen (e.g., ENDIF/ENDCASE/ELSE for parent)
fn parse_block(s: &str, i: &mut usize, end_tokens: &[&str]) -> Result<Vec<Node>> {
    let bytes = s.as_bytes();
    let mut out: Vec<Node> = Vec::new();
    let mut buf = String::new();
    while *i < s.len() {
        // Check for end tokens only at directive boundaries starting with '@'
        if bytes[*i] == b'@' {
            if let Some(tok) = peek_word(&s[*i + 1..]) {
                let upper = tok.to_ascii_uppercase();
                if end_tokens.iter().any(|t| t.eq_ignore_ascii_case(&upper)) {
                    break; // caller will consume the token
                }
            }
        }

        if starts_with_at(s, *i) {
            // flush text
            if !buf.is_empty() {
                out.push(Node::Text(std::mem::take(&mut buf)));
            }
            // parse directive
            let node = parse_at_directive(s, i)?;
            out.push(node);
            continue;
        }
        if starts_with2(s, *i, "{{") {
            // flush text
            if !buf.is_empty() {
                out.push(Node::Text(std::mem::take(&mut buf)));
            }
            // parse interpolation
            let start = *i; // position of '{{'
            *i += 2;
            let expr = read_balanced_until(s, i, "}}")?;
            out.push(Node::Interp {
                expr: expr.trim().to_string(),
                pos: start,
            });
            continue;
        }
        // default: accumulate one char
        buf.push(s[*i..].chars().next().unwrap());
        *i += s[*i..].chars().next().unwrap().len_utf8();
    }
    if !buf.is_empty() {
        out.push(Node::Text(buf));
    }
    Ok(out)
}

fn starts_with2(s: &str, i: usize, pat: &str) -> bool {
    s.get(i..i + pat.len()).map(|x| x == pat).unwrap_or(false)
}

fn starts_with_at(s: &str, i: usize) -> bool {
    if !starts_with2(s, i, "@") {
        return false;
    }
    // Require a letter to follow to avoid emails etc.
    s[i..]
        .chars()
        .nth(1)
        .map(|c| c.is_ascii_alphabetic() || c == '_')
        .unwrap_or(false)
}

fn peek_word(s: &str) -> Option<String> {
    let mut it = s.chars();
    let mut w = String::new();
    for c in it.by_ref() {
        if c.is_ascii_alphabetic() || c == '_' {
            w.push(c);
        } else {
            break;
        }
    }
    if w.is_empty() {
        None
    } else {
        Some(w)
    }
}

fn parse_at_directive(s: &str, i: &mut usize) -> Result<Node> {
    // expecting @NAME(args) or @ELSE / @ENDIF / @ENDCASE (handled by caller via end_tokens)
    debug_assert!(starts_with_at(s, *i));
    let start = *i; // position of '@'
    *i += 1; // skip @
    let name =
        read_ident(s, i).ok_or_else(|| BasilError("RENDER$: expected directive name".into()))?;
    let uname = name.to_ascii_uppercase();
    if uname == "ELSE"
        || uname == "ENDIF"
        || uname == "ENDCASE"
        || uname == "FORELSE"
        || uname == "ENDFOREACH"
        || uname == "ENDTIMES"
        || uname == "ENDFOR"
        || uname == "ENDWHILE"
    {
        // Should be handled by caller; back up to '@' for caller to consume token
        *i -= 1; // include '@'
        return Err(BasilError("RENDER$: unexpected block control token".into()));
    }
    // Special zero-arg directives (optionally accept empty parentheses)
    if uname == "BREAK" {
        skip_ws(s, i);
        if starts_with2(s, *i, "(") {
            *i += 1;
            let _ = read_args_until_rparen(s, i)?;
        }
        return Ok(Node::Break { pos: start });
    }
    if uname == "CONTINUE" {
        skip_ws(s, i);
        if starts_with2(s, *i, "(") {
            *i += 1;
            let _ = read_args_until_rparen(s, i)?;
        }
        return Ok(Node::Continue { pos: start });
    }

    // Read argument list in parentheses for the rest
    skip_ws(s, i);
    if !starts_with2(s, *i, "(") {
        return Err(BasilError(format!("RENDER$: expected '(' after @{}", name)));
    }
    *i += 1;
    let args_src = read_args_until_rparen(s, i)?; // advances i past ')'
    let args = split_top_level_commas(&args_src);

    match uname.as_str() {
        "IF" => {
            // Parse then-part until @ELSE or @ENDIF
            let then_part = parse_block(s, i, &["ELSE", "ENDIF"])?;
            // Now current position is at '@ELSE' or '@ENDIF'
            if !starts_with_at(s, *i) {
                return Err(BasilError("RENDER$: expected @ELSE or @ENDIF".into()));
            }
            *i += 1;
            let tok = read_ident(s, i).unwrap_or_default().to_ascii_uppercase();
            let else_part = if tok == "ELSE" {
                // parse else block until ENDIF
                let part = parse_block(s, i, &["ENDIF"])?;
                // consume @ENDIF
                if !starts_with_at(s, *i) {
                    return Err(BasilError("RENDER$: expected @ENDIF".into()));
                }
                *i += 1;
                let _ = read_ident(s, i);
                part
            } else if tok == "ENDIF" {
                Vec::new()
            } else {
                return Err(BasilError("RENDER$: expected @ELSE or @ENDIF".into()));
            };
            let cond = args.get(0).cloned().unwrap_or_default();
            Ok(Node::If {
                cond,
                then_part,
                else_part,
                pos: start,
            })
        }
        "CASE" => {
            // First arm is current args
            let mut arms: Vec<(String, Vec<Node>)> = Vec::new();
            let first_cond = args.get(0).cloned().unwrap_or_default();
            let first_body = parse_block(s, i, &["CASE", "ENDCASE"])?;
            arms.push((first_cond, first_body));
            loop {
                if !starts_with_at(s, *i) {
                    return Err(BasilError("RENDER$: expected @CASE or @ENDCASE".into()));
                }
                *i += 1;
                let tok = read_ident(s, i).unwrap_or_default().to_ascii_uppercase();
                if tok == "ENDCASE" {
                    break;
                }
                if tok != "CASE" {
                    return Err(BasilError("RENDER$: expected @CASE or @ENDCASE".into()));
                }
                skip_ws(s, i);
                if !starts_with2(s, *i, "(") {
                    return Err(BasilError("RENDER$: expected '(' in @CASE".into()));
                }
                *i += 1;
                let cond_src = read_args_until_rparen(s, i)?; // adv past ')'
                let cond = split_top_level_commas(&cond_src)
                    .get(0)
                    .cloned()
                    .unwrap_or_default();
                let body = parse_block(s, i, &["CASE", "ENDCASE"])?;
                arms.push((cond, body));
            }
            Ok(Node::Case { arms, pos: start })
        }
        // FOREACH families
        "FOREACH" | "FOREACH_KSORT" | "FOREACH_VSORT" | "FOREACH_KSORT_DESC"
        | "FOREACH_DSORT_DESC" => {
            // Parse header: ident IN expr
            let (var, expr) = parse_foreach_header(&args_src)?;
            let order = match uname.as_str() {
                "FOREACH_KSORT" => ForEachOrder::KSortAsc,
                "FOREACH_VSORT" => ForEachOrder::VSortAsc,
                "FOREACH_KSORT_DESC" => ForEachOrder::KSortDesc,
                "FOREACH_DSORT_DESC" => ForEachOrder::VSortDesc,
                _ => ForEachOrder::Natural,
            };
            // Parse body until @FORELSE or @ENDFOREACH
            let body = parse_block(s, i, &["FORELSE", "ENDFOREACH"])?;
            // consume @FORELSE or @ENDFOREACH
            if !starts_with_at(s, *i) {
                return Err(BasilError(
                    "RENDER$: expected @FORELSE or @ENDFOREACH".into(),
                ));
            }
            *i += 1;
            let tok = read_ident(s, i).unwrap_or_default().to_ascii_uppercase();
            let else_part = if tok == "FORELSE" {
                let part = parse_block(s, i, &["ENDFOREACH"])?;
                if !starts_with_at(s, *i) {
                    return Err(BasilError("RENDER$: expected @ENDFOREACH".into()));
                }
                *i += 1;
                let _ = read_ident(s, i);
                part
            } else if tok == "ENDFOREACH" {
                Vec::new()
            } else {
                return Err(BasilError(
                    "RENDER$: expected @FORELSE or @ENDFOREACH".into(),
                ));
            };
            Ok(Node::ForEach {
                var,
                enumerable: expr,
                body,
                else_part,
                order,
                pos: start,
            })
        }
        // TIMES
        "TIMES" => {
            let count = args
                .get(0)
                .cloned()
                .unwrap_or_else(|| args_src.trim().to_string());
            let body = parse_block(s, i, &["ENDTIMES"])?;
            if !starts_with_at(s, *i) {
                return Err(BasilError("RENDER$: expected @ENDTIMES".into()));
            }
            *i += 1;
            let _ = read_ident(s, i);
            Ok(Node::Times {
                count,
                body,
                pos: start,
            })
        }
        // FOR numeric: i% = start TO end [STEP step]
        "FOR" => {
            let (var, start_expr, end_expr, step_expr) = parse_for_header(&args_src)?;
            let body = parse_block(s, i, &["ENDFOR"])?;
            if !starts_with_at(s, *i) {
                return Err(BasilError("RENDER$: expected @ENDFOR".into()));
            }
            *i += 1;
            let _ = read_ident(s, i);
            Ok(Node::ForNum {
                var,
                start: start_expr,
                end: end_expr,
                step: step_expr,
                body,
                pos: start,
            })
        }
        // WHILE
        "WHILE" => {
            let cond = args
                .get(0)
                .cloned()
                .unwrap_or_else(|| args_src.trim().to_string());
            let body = parse_block(s, i, &["ENDWHILE"])?;
            if !starts_with_at(s, *i) {
                return Err(BasilError("RENDER$: expected @ENDWHILE".into()));
            }
            *i += 1;
            let _ = read_ident(s, i);
            Ok(Node::While {
                cond,
                body,
                pos: start,
            })
        }
        _ => Ok(Node::Call {
            name: uname,
            args,
            pos: start,
        }),
    }
}

// Parse FOREACH header: "var IN expr" (case-insensitive IN). Returns (var, expr)
fn parse_foreach_header(src: &str) -> Result<(String, String)> {
    let (mut i, mut in_pos) = (0usize, None);
    let bytes = src.as_bytes();
    // scan respecting quotes and parentheses
    let mut depth = 0i32;
    while i < src.len() {
        let ch = src[i..].chars().next().unwrap();
        if ch == '"' || ch == '\'' {
            // skip string
            let q = ch;
            i += ch.len_utf8();
            while i < src.len() {
                let c = src[i..].chars().next().unwrap();
                i += c.len_utf8();
                if c == q {
                    break;
                }
            }
            continue;
        }
        match ch {
            '(' => {
                depth += 1;
            }
            ')' => {
                depth -= 1;
            }
            _ => {}
        }
        if depth == 0 {
            // try to match IN at this position (case-insensitive), with word boundaries
            if i + 2 <= bytes.len() {
                // read a word
                if let Some(w) = peek_word(&src[i..]) {
                    if w.eq_ignore_ascii_case("IN") {
                        in_pos = Some(i);
                        break;
                    }
                }
            }
        }
        i += ch.len_utf8();
    }
    let in_idx = in_pos.ok_or_else(|| {
        BasilError("RENDER$: malformed @FOREACH header; expected 'var IN expr'".into())
    })?;
    let left = src[..in_idx].trim();
    // advance past IN token
    let mut j = in_idx;
    if let Some(w) = peek_word(&src[j..]) {
        j += w.len();
    } else {
        return Err(BasilError("RENDER$: malformed @FOREACH header".into()));
    }
    let right = src[j..].trim();
    if left.is_empty() || right.is_empty() {
        return Err(BasilError("RENDER$: malformed @FOREACH header".into()));
    }
    Ok((left.to_string(), right.to_string()))
}

// Parse FOR header: "var = start TO end [STEP step]"
fn parse_for_header(src: &str) -> Result<(String, String, String, Option<String>)> {
    // find '=' first at top level
    let mut depth = 0i32;
    let mut eq_pos: Option<usize> = None;
    let mut i = 0usize;
    while i < src.len() {
        let ch = src[i..].chars().next().unwrap();
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            '"' | '\'' => {
                let q = ch;
                i += ch.len_utf8();
                while i < src.len() {
                    let c = src[i..].chars().next().unwrap();
                    i += c.len_utf8();
                    if c == q {
                        break;
                    }
                }
                continue;
            }
            '=' if depth == 0 => {
                eq_pos = Some(i);
                break;
            }
            _ => {}
        }
        i += ch.len_utf8();
    }
    let eq =
        eq_pos.ok_or_else(|| BasilError("RENDER$: malformed @FOR header; expected '='".into()))?;
    let var = src[..eq].trim().to_string();
    // after '=' expect start ... TO ... [STEP ...]
    let rest = src[eq + 1..].trim();
    // find TO
    let mut k = 0usize;
    let mut to_pos: Option<usize> = None;
    depth = 0;
    while k < rest.len() {
        let ch = rest[k..].chars().next().unwrap();
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            '"' | '\'' => {
                let q = ch;
                k += ch.len_utf8();
                while k < rest.len() {
                    let c = rest[k..].chars().next().unwrap();
                    k += c.len_utf8();
                    if c == q {
                        break;
                    }
                }
                continue;
            }
            _ => {}
        }
        if depth == 0 {
            if let Some(w) = peek_word(&rest[k..]) {
                if w.eq_ignore_ascii_case("TO") {
                    to_pos = Some(k);
                    break;
                }
            }
        }
        k += ch.len_utf8();
    }
    let to_idx =
        to_pos.ok_or_else(|| BasilError("RENDER$: malformed @FOR header; expected 'TO'".into()))?;
    let start_expr = rest[..to_idx].trim().to_string();
    // move past TO keyword
    let mut m = to_idx;
    if let Some(w) = peek_word(&rest[m..]) {
        m += w.len();
    } else {
        return Err(BasilError("RENDER$: malformed @FOR header".into()));
    }
    let after_to = rest[m..].trim();
    // optional STEP
    let (end_expr, step_expr) = if let Some(step_idx) = find_keyword_top_level(after_to, "STEP") {
        let end_e = after_to[..step_idx].trim().to_string();
        let mut p = step_idx;
        if let Some(w) = peek_word(&after_to[p..]) {
            p += w.len();
        } else {
            return Err(BasilError("RENDER$: malformed @FOR header".into()));
        }
        let step_e = after_to[p..].trim().to_string();
        (
            end_e,
            if step_e.is_empty() {
                None
            } else {
                Some(step_e)
            },
        )
    } else {
        (after_to.to_string(), None)
    };
    if var.is_empty() || start_expr.is_empty() || end_expr.is_empty() {
        return Err(BasilError("RENDER$: malformed @FOR header".into()));
    }
    Ok((var, start_expr, end_expr, step_expr))
}

fn find_keyword_top_level(src: &str, kw: &str) -> Option<usize> {
    let mut i = 0usize;
    let mut depth = 0i32;
    while i < src.len() {
        let ch = src[i..].chars().next().unwrap();
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            '"' | '\'' => {
                let q = ch;
                i += ch.len_utf8();
                while i < src.len() {
                    let c = src[i..].chars().next().unwrap();
                    i += c.len_utf8();
                    if c == q {
                        break;
                    }
                }
                continue;
            }
            _ => {}
        }
        if depth == 0 {
            if let Some(w) = peek_word(&src[i..]) {
                if w.eq_ignore_ascii_case(kw) {
                    return Some(i);
                }
            }
        }
        i += ch.len_utf8();
    }
    None
}

fn read_ident(s: &str, i: &mut usize) -> Option<String> {
    skip_ws(s, i);
    let mut out = String::new();
    let mut it = s[*i..].char_indices();
    while let Some((off, c)) = it.next() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        } else {
            *i += off;
            return Some(out);
        }
    }
    *i = s.len();
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn skip_ws(s: &str, i: &mut usize) {
    while *i < s.len() {
        let c = s[*i..].chars().next().unwrap();
        if c.is_whitespace() {
            *i += c.len_utf8();
        } else {
            break;
        }
    }
}

fn read_balanced_until(s: &str, i: &mut usize, end: &str) -> Result<String> {
    // naive: read until first occurrence of end
    if let Some(pos) = s[*i..].find(end) {
        let out = &s[*i..*i + pos];
        *i += pos + end.len();
        Ok(out.to_string())
    } else {
        Err(BasilError(format!("RENDER$: expected '{}' to close", end)))
    }
}

fn read_args_until_rparen(s: &str, i: &mut usize) -> Result<String> {
    let mut depth = 1i32; // we start just after '('
    let mut out = String::new();
    let mut it = s[*i..].char_indices();
    while let Some((off, ch)) = it.next() {
        match ch {
            '(' => {
                depth += 1;
                out.push(ch);
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    *i += off + ch.len_utf8();
                    return Ok(out);
                } else {
                    out.push(ch);
                }
            }
            '"' | '\'' => {
                // read string literal until same quote (no escape sophistication)
                out.push(ch);
                let mut j = *i + off + ch.len_utf8();
                while j < s.len() {
                    let c = s[j..].chars().next().unwrap();
                    out.push(c);
                    j += c.len_utf8();
                    if c == ch {
                        break;
                    }
                }
                *i = j; // set base for next it cycle
                it = s[*i..].char_indices();
                continue;
            }
            _ => out.push(ch),
        }
    }
    Err(BasilError("RENDER$: unclosed ')'".into()))
}

fn split_top_level_commas(args_src: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut depth = 0i32;
    let mut it = args_src.chars().peekable();
    while let Some(ch) = it.next() {
        match ch {
            '"' | '\'' => {
                cur.push(ch);
                while let Some(c) = it.next() {
                    cur.push(c);
                    if c == ch {
                        break;
                    }
                }
            }
            '(' => {
                depth += 1;
                cur.push(ch);
            }
            ')' => {
                depth -= 1;
                cur.push(ch);
            }
            ',' if depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

// Detects a simple top-level assignment of the form: IDENT = <expr>
// Returns (true, Some(ident)) when detected; otherwise (false, None).
fn detect_simple_assignment(src: &str) -> (bool, Option<String>) {
    let s = src.trim();
    if s.is_empty() {
        return (false, None);
    }
    let mut i = 0usize;
    let mut depth: i32 = 0;
    let mut in_str: Option<char> = None;
    while i < s.len() {
        let ch = s[i..].chars().next().unwrap();
        // handle quotes
        if let Some(q) = in_str {
            if ch == '\\' {
                // skip escaped char inside string
                let adv = ch.len_utf8();
                i += adv;
                if i < s.len() {
                    i += s[i..].chars().next().unwrap().len_utf8();
                }
                continue;
            }
            if ch == q {
                in_str = None;
                i += ch.len_utf8();
                continue;
            }
            i += ch.len_utf8();
            continue;
        }
        match ch {
            '"' | '\'' => {
                in_str = Some(ch);
                i += ch.len_utf8();
                continue;
            }
            '(' => {
                depth += 1;
                i += ch.len_utf8();
                continue;
            }
            ')' => {
                depth -= 1;
                i += ch.len_utf8();
                continue;
            }
            '=' if depth == 0 => {
                // Exclude '==' and '<=' and '>=' operators
                let prev = if i == 0 {
                    None
                } else {
                    Some(s[..i].chars().last().unwrap())
                };
                let next = s[i + ch.len_utf8()..].chars().next();
                if matches!(prev, Some('<' | '>' | '=')) || matches!(next, Some('=')) {
                    i += ch.len_utf8();
                    continue;
                }
                // LHS is s[..i]
                let lhs = s[..i].trim();
                if is_simple_identifier(lhs) {
                    return (true, Some(lhs.to_string()));
                } else {
                    return (false, None);
                }
            }
            _ => {
                i += ch.len_utf8();
            }
        }
    }
    (false, None)
}

fn is_simple_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    // Allow A-Z a-z _ only for first char
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    // Rest may be alnum or '_', and optionally last char may be '$' or '%'
    let mut seen_any = false;
    let mut buf: Vec<char> = name.chars().collect();
    // Allow a trailing '$' or '%'
    if let Some(&last) = buf.last() {
        if last == '$' || last == '%' {
            buf.pop();
        }
    }
    for c in buf.into_iter().skip(1) {
        seen_any = true;
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return false;
        }
    }
    let _ = seen_any; // not strictly needed
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoopSignal {
    None,
    Break(usize),
    Continue(usize),
}

fn eval_nodes(vm: &mut VM, nodes: &Vec<Node>, ctx: &mut Ctx) -> Result<String> {
    let (s, sig) = eval_nodes_ctrl(vm, nodes, ctx)?;
    match sig {
        LoopSignal::None => Ok(s),
        LoopSignal::Break(pos) => Err(wrap_tmpl_error(
            &ctx.template,
            pos,
            &BasilError("RENDER$: @BREAK outside of loop".into()),
        )),
        LoopSignal::Continue(pos) => Err(wrap_tmpl_error(
            &ctx.template,
            pos,
            &BasilError("RENDER$: @CONTINUE outside of loop".into()),
        )),
    }
}

fn ensure_overlay(ctx: &mut Ctx) -> Rc<RefCell<HashMap<String, Value>>> {
    if ctx.overlay.is_none() {
        ctx.overlay = Some(Rc::new(RefCell::new(HashMap::new())));
    }
    ctx.overlay.as_ref().unwrap().clone()
}

fn set_overlay(ctx: &mut Ctx, key: &str, val: Value) -> Option<Value> {
    let ov = ensure_overlay(ctx);
    let mut map = ov.borrow_mut();
    map.insert(key.to_string(), val)
}
fn remove_overlay(ctx: &mut Ctx, key: &str, prev: Option<Value>) {
    let ov = ctx.overlay.as_ref().unwrap().clone();
    let mut map = ov.borrow_mut();
    if let Some(p) = prev {
        map.insert(key.to_string(), p);
    } else {
        map.remove(key);
    }
}

fn set_meta(
    ctx: &mut Ctx,
    idx: i64,
    count: i64,
) -> (
    Option<Value>,
    Option<Value>,
    Option<Value>,
    Option<Value>,
    Option<Value>,
) {
    let p1 = set_overlay(ctx, "_index%", Value::Int(idx));
    let p2 = set_overlay(ctx, "_number%", Value::Int(idx + 1));
    let p3 = set_overlay(ctx, "_count%", Value::Int(count));
    let p4 = set_overlay(ctx, "_first%", Value::Bool(idx == 0));
    let p5 = set_overlay(ctx, "_last%", Value::Bool(idx + 1 == count));
    (p1, p2, p3, p4, p5)
}
fn clear_meta(
    ctx: &mut Ctx,
    p: (
        Option<Value>,
        Option<Value>,
        Option<Value>,
        Option<Value>,
        Option<Value>,
    ),
) {
    remove_overlay(ctx, "_index%", p.0);
    remove_overlay(ctx, "_number%", p.1);
    remove_overlay(ctx, "_count%", p.2);
    remove_overlay(ctx, "_first%", p.3);
    remove_overlay(ctx, "_last%", p.4);
}

fn eval_nodes_ctrl(vm: &mut VM, nodes: &Vec<Node>, ctx: &mut Ctx) -> Result<(String, LoopSignal)> {
    let mut out = String::new();
    for n in nodes {
        match n {
            Node::Text(t) => out.push_str(t),
            Node::Interp { expr, pos } => match eval_basil_expr_with_fred(vm, expr, ctx) {
                Ok(v) => out.push_str(&value_to_string(&v)),
                Err(e) => return Err(wrap_tmpl_error(&ctx.template, *pos, &e)),
            },
            Node::Call { name, args, pos } => match eval_fred_call(vm, name, args, ctx) {
                Ok(s) => out.push_str(&s),
                Err(e) => return Err(wrap_tmpl_error(&ctx.template, *pos, &e)),
            },
            Node::If {
                cond,
                then_part,
                else_part,
                pos,
            } => match eval_basil_expr_with_fred(vm, cond, ctx) {
                Ok(v) => {
                    let (s, sig) = if truthy(&v) {
                        eval_nodes_ctrl(vm, then_part, ctx)?
                    } else {
                        eval_nodes_ctrl(vm, else_part, ctx)?
                    };
                    out.push_str(&s);
                    if sig != LoopSignal::None {
                        return Ok((out, sig));
                    }
                }
                Err(e) => return Err(wrap_tmpl_error(&ctx.template, *pos, &e)),
            },
            Node::Case { arms, pos } => {
                let mut done = false;
                for (csrc, body) in arms {
                    if done {
                        break;
                    }
                    match eval_basil_expr_with_fred(vm, csrc, ctx) {
                        Ok(v) => {
                            if truthy(&v) {
                                let (s, sig) = eval_nodes_ctrl(vm, body, ctx)?;
                                out.push_str(&s);
                                if sig != LoopSignal::None {
                                    return Ok((out, sig));
                                }
                                done = true;
                            }
                        }
                        Err(e) => return Err(wrap_tmpl_error(&ctx.template, *pos, &e)),
                    }
                }
            }
            Node::Break { pos } => {
                return Ok((out, LoopSignal::Break(*pos)));
            }
            Node::Continue { pos } => {
                return Ok((out, LoopSignal::Continue(*pos)));
            }
            Node::Times { count, body, pos } => {
                let n = as_i64(
                    eval_basil_expr_with_fred(vm, count, ctx)
                        .map_err(|e| wrap_tmpl_error(&ctx.template, *pos, &e))?,
                );
                let total = if n < 0 { 0 } else { n } as i64;
                let mut idx: i64 = 0;
                while idx < total {
                    let mp = set_meta(ctx, idx, total);
                    let (s, sig) = eval_nodes_ctrl(vm, body, ctx)?;
                    out.push_str(&s);
                    clear_meta(ctx, mp);
                    match sig {
                        LoopSignal::None => {}
                        LoopSignal::Continue(_) => {
                            idx += 1;
                            continue;
                        }
                        LoopSignal::Break(_) => {
                            break;
                        }
                    }
                    idx += 1;
                }
            }
            Node::ForNum {
                var,
                start,
                end,
                step,
                body,
                pos,
            } => {
                let start_i = as_i64(
                    eval_basil_expr_with_fred(vm, start, ctx)
                        .map_err(|e| wrap_tmpl_error(&ctx.template, *pos, &e))?,
                );
                let end_i = as_i64(
                    eval_basil_expr_with_fred(vm, end, ctx)
                        .map_err(|e| wrap_tmpl_error(&ctx.template, *pos, &e))?,
                );
                let step_i = match step {
                    Some(se) => {
                        let v = as_i64(
                            eval_basil_expr_with_fred(vm, se, ctx)
                                .map_err(|e| wrap_tmpl_error(&ctx.template, *pos, &e))?,
                        );
                        if v == 0 {
                            1
                        } else {
                            v
                        }
                    }
                    None => {
                        if end_i >= start_i {
                            1
                        } else {
                            -1
                        }
                    }
                };
                // compute count for meta
                let total = if (step_i > 0 && start_i > end_i) || (step_i < 0 && start_i < end_i) {
                    0
                } else {
                    let dist = (end_i - start_i).abs();
                    (dist / step_i.abs()) + 1
                };
                let prev = set_overlay(ctx, var, Value::Int(start_i));
                let mut idx: i64 = 0;
                let mut cur = start_i;
                while (step_i > 0 && cur <= end_i) || (step_i < 0 && cur >= end_i) {
                    // update loop var
                    let _ = set_overlay(ctx, var, Value::Int(cur));
                    let mp = set_meta(ctx, idx, total);
                    let (s, sig) = eval_nodes_ctrl(vm, body, ctx)?;
                    out.push_str(&s);
                    clear_meta(ctx, mp);
                    match sig {
                        LoopSignal::None => {}
                        LoopSignal::Continue(_) => {
                            cur += step_i;
                            idx += 1;
                            continue;
                        }
                        LoopSignal::Break(_) => {
                            break;
                        }
                    }
                    cur += step_i;
                    idx += 1;
                }
                remove_overlay(ctx, var, prev);
            }
            Node::While { cond, body, pos } => {
                let mut idx: i64 = 0;
                loop {
                    let v = eval_basil_expr_with_fred(vm, cond, ctx)
                        .map_err(|e| wrap_tmpl_error(&ctx.template, *pos, &e))?;
                    if !truthy(&v) {
                        break;
                    }
                    // Unknown total; set _count% to current 1-based for convenience
                    let mp = set_meta(ctx, idx, idx + 1);
                    let (s, sig) = eval_nodes_ctrl(vm, body, ctx)?;
                    out.push_str(&s);
                    clear_meta(ctx, mp);
                    match sig {
                        LoopSignal::None => {}
                        LoopSignal::Continue(_) => {
                            idx += 1;
                            continue;
                        }
                        LoopSignal::Break(_) => {
                            break;
                        }
                    }
                    idx += 1;
                }
            }
            Node::ForEach {
                var,
                enumerable,
                body,
                else_part,
                order,
                pos,
            } => {
                let val = eval_basil_expr_with_fred(vm, enumerable, ctx)
                    .map_err(|e| wrap_tmpl_error(&ctx.template, *pos, &e))?;
                // Collect iteration list of Values for the loop var
                let mut items: Vec<Value> = Vec::new();
                match &val {
                    Value::List(rc) => {
                        items.extend(rc.borrow().iter().cloned());
                    }
                    Value::Array(arr_rc) => {
                        let arr = arr_rc.as_ref();
                        if arr.dims.len() == 1 {
                            items.extend(arr.data.borrow().iter().cloned());
                        } else if arr.dims.len() == 2 && arr.dims[1] == 2 {
                            // keys are column 0
                            let rows = arr.dims[0];
                            let data = arr.data.borrow();
                            for r in 0..rows {
                                items.push(data[r * 2].clone());
                            }
                        } else {
                            // non-iterable shape → zero iterations
                        }
                    }
                    Value::Dict(rc) => {
                        let map = rc.borrow();
                        match order {
                            ForEachOrder::Natural => {
                                for k in map.keys() {
                                    items.push(Value::Str(k.clone()));
                                }
                            }
                            ForEachOrder::KSortAsc => {
                                let mut v: Vec<String> = map.keys().cloned().collect();
                                v.sort();
                                for k in v {
                                    items.push(Value::Str(k));
                                }
                            }
                            ForEachOrder::KSortDesc => {
                                let mut v: Vec<String> = map.keys().cloned().collect();
                                v.sort_by(|a, b| b.cmp(a));
                                for k in v {
                                    items.push(Value::Str(k));
                                }
                            }
                            ForEachOrder::VSortAsc => {
                                let mut v: Vec<(String, String)> = map
                                    .iter()
                                    .map(|(k, vv)| (k.clone(), format!("{}", vv)))
                                    .collect();
                                v.sort_by(|a, b| a.1.cmp(&b.1));
                                for (k, _) in v {
                                    items.push(Value::Str(k));
                                }
                            }
                            ForEachOrder::VSortDesc => {
                                let mut v: Vec<(String, String)> = map
                                    .iter()
                                    .map(|(k, vv)| (k.clone(), format!("{}", vv)))
                                    .collect();
                                v.sort_by(|a, b| b.1.cmp(&a.1));
                                for (k, _) in v {
                                    items.push(Value::Str(k));
                                }
                            }
                        }
                    }
                    _ => { /* non-iterable: zero iterations */ }
                }

                let total = items.len() as i64;
                if total == 0 {
                    // render else_part if any
                    let (s, sig) = eval_nodes_ctrl(vm, else_part, ctx)?;
                    out.push_str(&s);
                    if sig != LoopSignal::None {
                        return Ok((out, sig));
                    }
                } else {
                    let prev = set_overlay(ctx, var, Value::Null);
                    for (idx, it) in items.into_iter().enumerate() {
                        let it_idx = idx as i64;
                        let _ = set_overlay(ctx, var, it.clone());
                        // also set meta
                        let mp = set_meta(ctx, it_idx, total);
                        let (s, sig) = eval_nodes_ctrl(vm, body, ctx)?;
                        out.push_str(&s);
                        clear_meta(ctx, mp);
                        match sig {
                            LoopSignal::None => {}
                            LoopSignal::Continue(_) => {
                                continue;
                            }
                            LoopSignal::Break(_) => {
                                break;
                            }
                        }
                    }
                    remove_overlay(ctx, var, prev);
                }
            }
        }
    }
    Ok((out, LoopSignal::None))
}

fn value_to_string(v: &Value) -> String {
    format!("{}", v)
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Int(i) => *i != 0,
        Value::Num(n) => *n != 0.0,
        Value::Str(s) => !s.is_empty(),
        Value::Null => false,
        _ => true,
    }
}

fn eval_basil_expr_with_fred(vm: &mut VM, src: &str, ctx: &mut Ctx) -> Result<Value> {
    // Expand any nested Fred direct calls in the expression into string literals
    let expanded = expand_fred_in_expr(vm, src, ctx)?;
    eval_basil_expr(vm, &expanded, &ctx.overlay)
}

fn expand_fred_in_expr(vm: &mut VM, src: &str, ctx: &mut Ctx) -> Result<String> {
    let mut out = String::new();
    let mut i = 0usize;
    while i < src.len() {
        if starts_with_at(src, i) {
            // parse directive here and evaluate as inline direct-eval
            let mut j = i + 1;
            let name = read_ident(src, &mut j)
                .unwrap_or_default()
                .to_ascii_uppercase();
            // Only expand non-flow-control (no IF/CASE) here
            if name == "IF"
                || name == "CASE"
                || name == "ELSE"
                || name == "ENDIF"
                || name == "ENDCASE"
            {
                // Not supported inside expressions; treat literally
                out.push(src[i..].chars().next().unwrap());
                i += src[i..].chars().next().unwrap().len_utf8();
                continue;
            }
            skip_ws(src, &mut j);
            if !starts_with2(src, j, "(") {
                out.push('@');
                i += 1;
                continue;
            }
            j += 1;
            let args_src = read_args_until_rparen(src, &mut j)?; // j at ')'
            let args = split_top_level_commas(&args_src);
            let s = eval_fred_call(vm, &name, &args, ctx)?;
            out.push_str(&basil_string_literal(&s));
            i = j; // after ')'
            continue;
        }
        out.push(src[i..].chars().next().unwrap());
        i += src[i..].chars().next().unwrap().len_utf8();
    }
    Ok(out)
}

fn basil_string_literal(s: &str) -> String {
    // Quote with double quotes and escape internal quotes and backslashes minimally
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => {
                out.push('\\');
                out.push('"');
            }
            '\\' => {
                out.push('\\');
                out.push('\\');
            }
            '\n' => {
                out.push_str("\n");
            }
            '\r' => {
                out.push_str("\r");
            }
            '\t' => {
                out.push_str("\t");
            }
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn eval_basil_expr(
    vm: &mut VM,
    expr_src: &str,
    overlay: &Option<Rc<RefCell<HashMap<String, Value>>>>,
) -> Result<Value> {
    use basil_compiler::compile as compile_basil;
    use basil_parser::parse as parse_basil;
    // Build DECLARE FUNCTION prototypes prelude for known user-defined functions so the compiler
    // resolves NAME(...) as a function call, not array access. The compiler now records DECLARE
    // names during a pre-scan and emits no code for them.
    let (names, values) = vm.globals_snapshot();
    let mut prelude = String::new();
    for (idx, name) in names.iter().enumerate() {
        if let Value::Func(f) = &values[idx] {
            let arity = f.arity as usize;
            // Build dummy parameter list (no type suffixes needed)
            let params = if arity == 0 {
                String::new()
            } else {
                (1..=arity)
                    .map(|i| format!("p{}", i))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            // DECLARE prototype only — no body
            prelude.push_str(&format!("DECLARE FUNCTION {}({})\n", name, params));
        }
    }
    // Detect simple assignment form like: ident = <expr>. If present, execute it as a statement
    // in the child VM and then propagate the new value back to the parent VM and overlay.
    let (is_assign, lhs_ident) = detect_simple_assignment(expr_src);
    let code = if is_assign {
        // Execute the assignment as a statement; make __RENDER_TMP an empty string so that
        // interpolation prints nothing for assignment side effects.
        format!("{}\n{};\nLET __RENDER_TMP = \"\";", prelude, expr_src)
    } else {
        format!("{}\nLET __RENDER_TMP = ({});", prelude, expr_src)
    };
    let ast = parse_basil(&code)?;
    let prog = compile_basil(&ast)?;
    let mut child = VM::new(prog.clone());
    if let Some(sp) = &vm.script_path {
        child.set_script_path(sp.clone());
    }
    // Seed globals from parent (functions and data)
    for (idx, name) in names.iter().enumerate() {
        let _ = child.set_global_by_name(name, values[idx].clone());
    }
    // Overlay context dictionary variables, with suffix mapping and precedence over globals
    if let Some(dict_rc) = overlay {
        let dict = dict_rc.borrow();
        for (k, v) in dict.iter() {
            inject_overlay_var(&mut child, k, v.clone());
        }
    }
    child.run()?;
    // If this was an assignment to a simple identifier, read the assigned value from child and
    // propagate it back to the parent global (and overlay map if present).
    if is_assign {
        if let Some(var) = lhs_ident.as_deref() {
            if let Some(newv) = child.get_global_by_name(var) {
                // Update parent VM global if it exists
                let _ = vm.set_global_by_name(var, newv.clone());
                // Update overlay entry if present (only exact key, no aliasing here)
                if let Some(ov) = overlay {
                    let mut map = ov.borrow_mut();
                    if map.contains_key(var) {
                        map.insert(var.to_string(), newv.clone());
                    }
                }
            }
        }
    }
    // locate result global
    let mut idx_opt: Option<usize> = None;
    for (i, name) in prog.globals.iter().enumerate() {
        if name == "__RENDER_TMP" {
            idx_opt = Some(i);
            break;
        }
    }
    let idx =
        idx_opt.ok_or_else(|| BasilError("RENDER internal error: result not found".into()))?;
    let val = child.globals.get(idx).cloned().unwrap_or(Value::Null);
    Ok(val)
}

fn eval_fred_call(vm: &mut VM, name: &str, args: &Vec<String>, ctx: &mut Ctx) -> Result<String> {
    let nm = name.to_ascii_uppercase();
    match nm.as_str() {
        // Direct evaluation built-ins
        "LEFT" => {
            let s = as_string(eval_basil_expr_with_fred(
                vm,
                args.get(0)
                    .ok_or_else(|| BasilError("LEFT expects 2 args".into()))?,
                ctx,
            )?);
            let n = as_i64(eval_basil_expr_with_fred(
                vm,
                args.get(1)
                    .ok_or_else(|| BasilError("LEFT expects 2 args".into()))?,
                ctx,
            )?);
            Ok(left_chars(&s, n))
        }
        "RIGHT" => {
            let s = as_string(eval_basil_expr_with_fred(
                vm,
                args.get(0)
                    .ok_or_else(|| BasilError("RIGHT expects 2 args".into()))?,
                ctx,
            )?);
            let n = as_i64(eval_basil_expr_with_fred(
                vm,
                args.get(1)
                    .ok_or_else(|| BasilError("RIGHT expects 2 args".into()))?,
                ctx,
            )?);
            Ok(right_chars(&s, n))
        }
        "MID" => {
            let s = as_string(eval_basil_expr_with_fred(
                vm,
                args.get(0)
                    .ok_or_else(|| BasilError("MID expects 3 args".into()))?,
                ctx,
            )?);
            let start = as_i64(eval_basil_expr_with_fred(
                vm,
                args.get(1)
                    .ok_or_else(|| BasilError("MID expects 3 args".into()))?,
                ctx,
            )?);
            let len = as_i64(eval_basil_expr_with_fred(
                vm,
                args.get(2)
                    .ok_or_else(|| BasilError("MID expects 3 args".into()))?,
                ctx,
            )?);
            Ok(mid_chars(&s, start, len))
        }
        "TRIM" => {
            let s = as_string(eval_basil_expr_with_fred(
                vm,
                args.get(0)
                    .ok_or_else(|| BasilError("TRIM expects 1 arg".into()))?,
                ctx,
            )?);
            Ok(s.trim().to_string())
        }
        "URLENCODE" => {
            let s = as_string(eval_basil_expr_with_fred(
                vm,
                args.get(0)
                    .ok_or_else(|| BasilError("URLENCODE expects 1 arg".into()))?,
                ctx,
            )?);
            Ok(vm.url_encode_form(&s))
        }
        "ENV" => {
            let name = as_string(eval_basil_expr_with_fred(
                vm,
                args.get(0)
                    .ok_or_else(|| BasilError("ENV expects 1 arg".into()))?,
                ctx,
            )?);
            Ok(env::var(&name).unwrap_or_else(|_| "null".into()))
        }
        "SERVER" => {
            let name = as_string(eval_basil_expr_with_fred(
                vm,
                args.get(0)
                    .ok_or_else(|| BasilError("SERVER expects 1 arg".into()))?,
                ctx,
            )?);
            Ok(env::var(&name).unwrap_or_else(|_| "null".into()))
        }
        "REQUEST" => {
            let name = as_string(eval_basil_expr_with_fred(
                vm,
                args.get(0)
                    .ok_or_else(|| BasilError("REQUEST expects 1 arg".into()))?,
                ctx,
            )?);
            Ok(get_request_param(vm, &name).unwrap_or_else(|| "null".into()))
        }
        "SESSION" => {
            let _name = as_string(eval_basil_expr_with_fred(
                vm,
                args.get(0)
                    .ok_or_else(|| BasilError("SESSION expects 1 arg".into()))?,
                ctx,
            )?);
            Ok("null".into()) // stub for non-CGI/sessionless contexts
        }
        "INCLUDE" => {
            if ctx.depth >= ctx.max_depth {
                return Err(BasilError("RENDER$: include depth exceeded".into()));
            }
            let path = as_string(eval_basil_expr_with_fred(
                vm,
                args.get(0)
                    .ok_or_else(|| BasilError("INCLUDE expects 1 arg".into()))?,
                ctx,
            )?);
            let content = read_include(vm, &path)?;
            ctx.depth += 1;
            let rendered = render_inner(vm, &content, ctx)?;
            ctx.depth -= 1;
            Ok(rendered)
        }
        _ => {
            // Fallback: call Basil user-defined function NAME(args...)
            let call_src = format!("{}({})", nm, args.join(", "));
            let v = eval_basil_expr_with_fred(vm, &call_src, ctx)?;
            Ok(value_to_string(&v))
        }
    }
}

// Inject a single overlay variable into the child VM with suffix mapping.
fn inject_overlay_var(child: &mut VM, key: &str, val: Value) {
    // Always try the exact key first
    let _ = child.set_global_by_name(key, val.clone());
    // Only add extra suffixed alias if key does not already end with a type suffix
    let upper = key.to_ascii_uppercase();
    let has_suffix = upper.ends_with('$') || upper.ends_with('%');
    if !has_suffix {
        match &val {
            Value::Str(_) => {
                let alias = format!("{}$", key);
                let _ = child.set_global_by_name(&alias, val);
            }
            Value::Int(_) => {
                let alias = format!("{}%", key);
                let _ = child.set_global_by_name(&alias, val);
            }
            _ => { /* no extra alias */ }
        }
    }
}

fn as_string(v: Value) -> String {
    match v {
        Value::Str(s) => s,
        Value::Null => String::new(),
        other => format!("{}", other),
    }
}
fn as_i64(v: Value) -> i64 {
    match v {
        Value::Int(i) => i,
        Value::Num(n) => n.trunc() as i64,
        Value::Bool(b) => {
            if b {
                1
            } else {
                0
            }
        }
        Value::Str(s) => s.parse::<i64>().unwrap_or(0),
        _ => 0,
    }
}

fn left_chars(s: &str, n: i64) -> String {
    if n <= 0 {
        return String::new();
    }
    s.chars().take(n as usize).collect()
}
fn right_chars(s: &str, n: i64) -> String {
    if n <= 0 {
        return String::new();
    }
    let len = s.chars().count() as i64;
    let take = if n > len { len } else { n } as usize;
    s.chars()
        .rev()
        .take(take)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}
fn mid_chars(s: &str, start1: i64, len: i64) -> String {
    if len <= 0 {
        return String::new();
    }
    let start0 = if start1 <= 0 { 0 } else { start1 - 1 } as usize;
    let mut it = s.chars();
    let _ = it.by_ref().take(start0).for_each(|_| ());
    it.take(len as usize).collect()
}

fn read_include(vm: &VM, path: &str) -> Result<String> {
    let p = if PathBuf::from(path).is_absolute() {
        PathBuf::from(path)
    } else {
        let base = vm.script_dir().unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });
        PathBuf::from(base).join(path)
    };
    match fs::read_to_string(&p) {
        Ok(s) => Ok(s),
        Err(_) => Ok("null".into()),
    }
}

fn get_request_param(vm: &mut VM, name: &str) -> Option<String> {
    // Build caches
    vm.ensure_get_params();
    vm.ensure_post_params();
    // Prefer POST then GET like many frameworks do; or choose GET then POST
    let key = format!("{}=", name);
    if let Some(v) = &vm.post_params_cache {
        for p in v {
            if p.starts_with(&key) {
                return Some(p[key.len()..].to_string());
            }
        }
    }
    if let Some(v) = &vm.get_params_cache {
        for p in v {
            if p.starts_with(&key) {
                return Some(p[key.len()..].to_string());
            }
        }
    }
    None
}

// --- Error reporting helpers ---
fn wrap_tmpl_error(template: &str, pos: usize, err: &BasilError) -> BasilError {
    let (line, col) = line_col(template, pos);
    BasilError(format!(
        "RENDER$ error at template {}:{}: {}",
        line, col, err
    ))
}

fn line_col(s: &str, pos: usize) -> (usize, usize) {
    let mut line: usize = 1;
    let mut col: usize = 1;
    for (i, ch) in s.char_indices() {
        if i >= pos {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            // Treat CR as part of line if present, but do not double-count columns on CRLF
            if ch != '\r' {
                col += 1;
            }
        }
    }
    (line, col)
}
