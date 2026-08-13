use basil_web::{self, cgi, util};
use std::path::PathBuf;

#[test]
fn safe_join_denies_traversal() {
    let root = tempfile::tempdir().unwrap();
    let rootp = root.path();
    std::fs::create_dir_all(rootp.join("a")).unwrap();
    let ok = util::safe_join(rootp, "/a/index.html").unwrap();
    assert!(ok.starts_with(rootp));
    let bad = util::safe_join(rootp, "/../etc/passwd");
    assert!(bad.is_err());
}

#[test]
fn cgi_header_parsing() {
    let raw = b"Status: 200 OK\r\nContent-Type: text/plain\r\nX-Test: 1\r\n\r\nHello";
    let so = cgi::split_headers_and_body(raw);
    assert_eq!(so.status.unwrap(), http::StatusCode::OK);
    assert_eq!(so.headers.get("content-type").unwrap(), "text/plain");
    assert_eq!(&so.body[..], b"Hello");
}

#[test]
fn safe_join_allows_root_relative_nonexistent() {
    let root = tempfile::tempdir().unwrap();
    let rootp = root.path();
    // Intentionally do not create favicon.ico
    let p = util::safe_join(rootp, "/favicon.ico")
        .expect("should allow root-relative non-existing path");
    assert!(p.starts_with(rootp));
}
