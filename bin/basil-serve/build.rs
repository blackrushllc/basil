#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    let ver = env!("CARGO_PKG_VERSION");
    res.set("FileVersion", ver);
    res.set("ProductVersion", ver);
    res.set("ProductName", "Basil BASIC");
    res.set("FileDescription", "Basil Local Web Server (basil-serve)");
    res.set("CompanyName", "Blackrush LLC");
    res.set("LegalCopyright", "© Blackrush LLC");
    let icon_path = "../../favicon.ico";
    if std::path::Path::new(icon_path).exists() {
        res.set_icon(icon_path);
    }
    res.compile().expect("Failed to embed Windows resources for basil-serve");
}

#[cfg(not(windows))]
fn main() {}
