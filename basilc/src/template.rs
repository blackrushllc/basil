use std::fmt;

#[derive(Debug, Clone, Default)]
pub struct Directives {
    pub cgi_no_header: bool,
    pub cgi_default_header: Option<String>,
    pub short_tags_on: bool,
    pub reserved_basil_dev: bool,
    pub reserved_basil_debug: bool,
}

#[derive(Debug, Clone)]
pub struct PrecompileResult {
    pub basil_source: String,
    pub directives: Directives,
    // future: source map entries
}

#[derive(Debug)]
pub enum TplError {
    Msg(String),
}
impl fmt::Display for TplError { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { match self { TplError::Msg(s) => write!(f, "{}", s) } } }
impl std::error::Error for TplError {}

pub fn parse_directives_and_bom(src: &str) -> (Directives, usize) {
    // Consume UTF-8 BOM if present (U+FEFF char)
    let mut i = 0usize;
    if src.as_bytes().starts_with(&[0xEF, 0xBB, 0xBF]) {
        i = 3;
    }
    let mut dir = Directives::default();
    // Only directives at top-of-file prelude; stop at first non-directive line
    loop {
        if i >= src.len() { break; }
        let rest = &src[i..];
        if !rest.starts_with('#') { break; }
        // read this line
        let line_end = rest.find('\n').map(|n| i + n + 1).unwrap_or(src.len());
        let line = &src[i..line_end].trim_end_matches(['\r','\n']);
        // match directives
        if line.starts_with("#CGI_NO_HEADER") { dir.cgi_no_header = true; }
        else if let Some(rest) = line.strip_prefix("#CGI_DEFAULT_HEADER") {
            // Expect quoted string
            if let Some(qpos) = rest.find('"') {
                let after = &rest[qpos+1..];
                if let Some(endq) = after.rfind('"') {
                    let val = &after[..endq];
                    dir.cgi_default_header = Some(val.to_string());
                }
            }
        }
        else if line.starts_with("#CGI_SHORT_TAGS_ON") { dir.short_tags_on = true; }
        else if line.starts_with("#BASIL_DEV") { dir.reserved_basil_dev = true; }
        else if line.starts_with("#BASIL_DEBUG") { dir.reserved_basil_debug = true; }
        else {
            // Unknown # line at prelude: ignore (kept as prelude semantics)
        }
        i = line_end;
    }
    (dir, i)
}

pub fn precompile_template(src: &str) -> Result<PrecompileResult, TplError> {
    let (directives, mut i) = parse_directives_and_bom(src);
    let bytes = src.as_bytes();
    let mut out = String::new();


    // Helper to append PRINT of raw text
    let emit_text = |text: &str, out: &mut String| {
        if text.is_empty() { return; }
        let mut s = String::with_capacity(text.len()+2);
        s.push('"');
        for ch in text.chars() {
            match ch {
                '"' => s.push_str("\\\""),
                '\\' => s.push_str("\\\\"),
                '\n' => s.push_str("\\n"),
                '\r' => s.push_str("\\r"),
                '\t' => s.push_str("\\t"),
                _ => s.push(ch),
            }
        }
        s.push('"');
        out.push_str("PRINT ");
        out.push_str(&s);
        out.push_str(";\n");
    };

    // Scan TEXT/ECHO/CODE
    let mut text_start = i; // start of current TEXT segment
    while i < bytes.len() {
        // find next '<?' by scanning forward
        let mut scan = i;
        let mut ltq_opt: Option<usize> = None;
        while scan + 1 < bytes.len() {
            if bytes[scan] == b'<' && bytes[scan+1] == b'?' { ltq_opt = Some(scan); break; }
            scan += 1;
        }
        let Some(ltq) = ltq_opt else { break; };
        // Emit text up to ltq
        if ltq > text_start {
            let text = &src[text_start..ltq];
            emit_text(text, &mut out);
        }
        // Determine tag type
        if ltq + 2 > bytes.len() { return Err(TplError::Msg("unterminated tag opener".into())); }
        let after = ltq + 2;
        if after >= bytes.len() { return Err(TplError::Msg("unterminated tag".into())); }
        // Echo shorthand
        if bytes.get(after) == Some(&b'=') {
            // Find closing '?>' honoring Basil string/comment syntax
            let (end, _) = find_closing(src, after+1)?; // start after '='<
            let expr = &src[after+1 .. end];
            // Basic validation: no semicolons or block keywords
            if expr.contains(';') || contains_kw(expr, &["BEGIN","END","WHILE","FOR","IF","ELSE","FUNC"]) {
                return Err(TplError::Msg("Echo block accepts a single expression only".into()));
            }
            let expr_trim = expr.trim();
            out.push_str("PRINT ("); out.push_str(expr_trim); out.push_str(");\n");
            i = end + 2; // skip '?>'
            text_start = i;
            continue;
        }
        // Code block <?basil ... ?> or short <?bas ... ?> if enabled
        let rest = &src[after..];
        if rest.starts_with("basil") || (directives.short_tags_on && rest.starts_with("bas")) {
            // compute start of code content
            let code_start = if rest.starts_with("basil") { after + 5 } else { after + 3 };
            // skip optional whitespace
            let mut cs = code_start;
            while cs < src.len() && src.as_bytes()[cs].is_ascii_whitespace() { cs += 1; }
            // Find '?>' honoring strings/comments
            let (end, _) = find_closing(src, cs)?;
            let code = &src[cs..end];
            out.push_str(code);
            let code_trim = code.trim_end();
            if !code_trim.ends_with(';') && !code_trim.is_empty() { out.push_str(";\n"); }
            else { out.push('\n'); }
            i = end + 2; text_start = i;
            continue;
        }
        // Illegal bare '<?...'
        return Err(TplError::Msg("Illegal bare '<? ... ?>'. Use <?basil ... ?>, <?bas ... ?> (with #CGI_SHORT_TAGS_ON), or <?= expr ?>.".into()));
    }
    // Tail text
    if text_start < src.len() { emit_text(&src[text_start..], &mut out); }

    Ok(PrecompileResult { basil_source: out, directives })
}

fn contains_kw(s: &str, kws: &[&str]) -> bool {
    let up = s.to_ascii_uppercase();
    for k in kws { if up.contains(k) { return true; } }
    false
}

// Find closing '?>' from index `start` in SRC, skipping over strings and comments.
// Returns (index_of_'?' in '?>', state)
fn find_closing(src: &str, start: usize) -> Result<(usize, ()), TplError> {
    let bytes = src.as_bytes();
    let mut i = start;
    let mut in_str = false;
    let mut in_slash_comment = false;
    let mut in_tick_comment = false;
    while i+1 < bytes.len() {
        let c = bytes[i];
        let n = bytes[i+1];
        if in_str {
            if c == b'\\' { i += 2; continue; }
            if c == b'"' { in_str = false; i+=1; continue; }
            i+=1; continue;
        }
        if in_slash_comment {
            if c == b'\n' { in_slash_comment = false; }
            i+=1; continue;
        }
        if in_tick_comment {
            if c == b'\n' { in_tick_comment = false; }
            i+=1; continue;
        }
        // Enter comment?
        if c == b'/' && n == b'/' { in_slash_comment = true; i+=2; continue; }
        if c == b'\'' { in_tick_comment = true; i+=1; continue; }
        if c == b'"' { in_str = true; i+=1; continue; }
        // Check for '?>'
        if c == b'?' && n == b'>' { return Ok((i, ())); }
        i+=1;
    }
    Err(TplError::Msg("Unterminated block: expected '?>'".into()))
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_rendering() {
        let tpl = "Hello, world!\n";
        let pre = precompile_template(tpl).unwrap();
        assert!(pre.basil_source.contains("PRINT \"Hello, world!\\n\";"));
    }

    #[test]
    fn code_block_execution() {
        let tpl = "A <?basil PRINT 1+1; ?> B";
        let pre = precompile_template(tpl).unwrap();
        // Should include the code block as-is
        assert!(pre.basil_source.contains("PRINT 1+1;"));
        // And the following text segment
        assert!(pre.basil_source.contains("PRINT \" B\";"));
    }

    #[test]
    fn echo_shorthand_and_html() {
        let tpl = "<?= HTML(\"<x>\") ?>";
        let pre = precompile_template(tpl).unwrap();
        assert!(pre.basil_source.contains("PRINT (HTML(\"<x>\"));"));
    }

    #[test]
    fn short_tags_toggle() {
        let tpl = "<?bas PRINT 1; ?>";
        let _err = precompile_template(tpl).unwrap_err();
        let tpl_on = "#CGI_SHORT_TAGS_ON\n<?bas PRINT 1; ?>";
        let pre = precompile_template(tpl_on).unwrap();
        assert!(pre.basil_source.contains("PRINT 1;"));
    }

    #[test]
    fn directive_prelude_and_bom_consumed() {
        let tpl = "\u{feff}#CGI_NO_HEADER\n#BASIL_DEBUG\nHello";
        let pre = precompile_template(tpl).unwrap();
        assert!(pre.directives.cgi_no_header);
        assert!(!pre.basil_source.contains("#CGI_NO_HEADER"));
        assert!(pre.basil_source.contains("PRINT \"Hello\";"));
    }

    #[test]
    fn delimiter_robustness() {
        let tpl = "<?basil PRINT \"hello ?> world\"; ?>";
        let pre = precompile_template(tpl).unwrap();
        assert!(pre.basil_source.contains("PRINT \"hello ?> world\";"));
    }

    #[test]
    fn multiple_blocks_with_html_between() {
        let tpl = "<?basil PRINT 1; ?>\n<!doctype html>\n<div>text</div>\n<?basil PRINT 2; ?>";
        let pre = precompile_template(tpl).unwrap();
        assert!(pre.basil_source.contains("PRINT 1;"));
        assert!(pre.basil_source.contains("PRINT 2;"));
    }

    #[test]
    fn cgi_example_second_block_executes_crlf_and_indent() {
        let tpl = "#CGI_NO_HEADER\r\n<?basil\r\n  PRINT \"Status: 200 OK\\r\\n\";\r\n  PRINT \"Content-Type: text/html; charset=utf-8\\r\\n\\r\\n\";\r\n?>\r\n<!doctype html>\r\n<html lang=\"en\">\r\n<head>\r\n  <meta charset=\"utf-8\">\r\n  <title>Basil CGI Demo</title>\r\n</head>\r\n<body>\r\n  <h1>Hello, World</h1>\r\n  <p>This page is rendered by a Basil CGI template.</p>\r\n\r\n  <h2>Request parameters</h2>\r\n  <p>Any GET or POST parameters will be listed below.</p>\r\n  <ul>\r\n  <?basil\r\n    FOR EACH p$ IN REQUEST$()\r\n      PRINT \"<li>\" + HTML$(p$) + \"</li>\\n\";\r\n    NEXT\r\n  ?>\r\n  </ul>\r\n</body>\r\n</html>\r\n";
        let pre = precompile_template(tpl).unwrap();
        // Should not leave raw tags in output
        assert!(!pre.basil_source.contains("<?basil"));
        // Should include the FOR EACH code
        assert!(pre.basil_source.contains("FOR EACH p$ IN REQUEST$()"));
        assert!(pre.basil_source.contains("NEXT"));
    }

}
