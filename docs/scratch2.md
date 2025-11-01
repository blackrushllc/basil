Perfect 😎 — here’s your **final, polished Junie prompt**, clean, unambiguous, and ready to paste straight into Junie Ultimate.
Everything is updated for **.NET 8 LTS**, includes the `winres` integration, the workspace clarification, and version-binding polish.

---

## ✅ **Final Junie Prompt**

````
Junie, we are going to finalize the Windows MSI installer and version system for the Basil project.

Please keep the WiX v4 MSI setup exactly as previously designed, but target a supported .NET SDK:

- Use **.NET 8 SDK (LTS)** or newer — not .NET 6.  
- Continue using `WixToolset.Sdk` as-is; rely on `dotnet restore/build` to fetch it.  
- Update Product.wxs to use the project-root icon:
  ```xml
  <Icon Id="AppIcon" SourceFile="$(var.RepoRoot)\favicon.ico" />
  <Property Id="ARPPRODUCTICON" Value="AppIcon" />
````

* Preserve all existing installer behavior:

    * Install **basilc.exe**, **bcc.exe**, **basil-serve.exe** → `ProgramFiles64Folder\Basil\bin`
    * Append that folder to the **system PATH**
    * Install **/docs** + `README.md` + `WHATS_NEW.md` → `ProgramFiles64Folder\Basil\docs`
    * Optionally install **/examples** → user-chosen folder (default `Documents\Basil\Examples`)

        * Exclude `*.basilx`, `.env`, any file/dir starting with `.` or `_`, `examples/.basil`, and `examples/basilbasic.com/.basilcache`
    * After install, show a checkbox **“Open What’s New after installation”** (checked by default)
      that opens `https://basilbasic.com/basil/` in the default browser.
* Keep the staging/harvest pipeline in `/scripts/build-windows-msi.ps1` and `/installer/windows/wix/Basil.wixproj`.
* MSI output must appear in `installer/windows/wix/bin/Release/`.

---

### 🧩 Add Real Windows Version Resources

We need each Windows EXE to embed a real `VERSIONINFO` block so the MSI can bind its version from `basilc.exe`.

**Targets:**

1. basilc
2. bcc
3. basil-serve

**Overall goals**

* Each EXE should have matching `FileVersion` and `ProductVersion` from `CARGO_PKG_VERSION` (e.g. `1.0.0`).
* Include standard fields: `CompanyName`, `ProductName`, `FileDescription`, `LegalCopyright`.
* Optionally embed the icon (`favicon.ico`) for Windows builds.

> 💡 Assume these crates live inside `/crates/`; use `"../../favicon.ico"` as the relative icon path unless the layout differs.
> If the crates are flat at repo root, change it to `"../favicon.ico"`.
> If the project uses a Cargo workspace, keep all three crate versions synchronized with the workspace root `[workspace.package] version`.

---

### Implementation Steps

1️⃣ Add the `winres` build dependency to each executable crate:
*(pin to a stable version for reproducibility)*

```toml
[build-dependencies]
winres = "0.1.12"
```

2️⃣ Create a `build.rs` file in each EXE crate using these templates:

**basilc/build.rs**

```rust
#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    let ver = env!("CARGO_PKG_VERSION");
    res.set("FileVersion", ver);
    res.set("ProductVersion", ver);
    res.set("ProductName", "Basil BASIC");
    res.set("FileDescription", "Basil BASIC Interpreter (basilc)");
    res.set("CompanyName", "Blackrush LLC");
    res.set("LegalCopyright", "© Blackrush LLC");
    let icon_path = "../../favicon.ico";
    if std::path::Path::new(icon_path).exists() {
        res.set_icon(icon_path);
    }
    res.compile().expect("Failed to embed Windows resources for basilc");
}

#[cfg(not(windows))]
fn main() {}
```

**bcc/build.rs**

```rust
#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    let ver = env!("CARGO_PKG_VERSION");
    res.set("FileVersion", ver);
    res.set("ProductVersion", ver);
    res.set("ProductName", "Basil BASIC");
    res.set("FileDescription", "Basil BASIC Compiler (bcc)");
    res.set("CompanyName", "Blackrush LLC");
    res.set("LegalCopyright", "© Blackrush LLC");
    let icon_path = "../../favicon.ico";
    if std::path::Path::new(icon_path).exists() {
        res.set_icon(icon_path);
    }
    res.compile().expect("Failed to embed Windows resources for bcc");
}

#[cfg(not(windows))]
fn main() {}
```

**basil-serve/build.rs**

```rust
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
```

3️⃣ Confirm each crate produces its binary correctly on Windows.
If needed, verify `[[bin]]` in each `Cargo.toml`.

4️⃣ Build with explicit features:

```powershell
cargo build -p basilc --release --features obj-all
cargo build -p bcc --release
cargo build -p basil-serve --release
```

5️⃣ In `installer/windows/wix/Product.wxs`, set version binding and upgrade handling:

```xml
<Package Name="Basil BASIC"
         Manufacturer="Blackrush LLC"
         Version="!(bind.fileVersion.basilc_exe)"
         UpgradeCode="{F1C3EF7D-ABCD-4F11-9C77-111122223333}"
         ProductCode="*"
         Language="1033"
         InstallerVersion="500"
         Scope="perMachine"
         Compressed="yes"
         Platform="x64">
  <MajorUpgrade DowngradeErrorMessage="A newer version of Basil is already installed." />
  ...
</Package>
```

⚙ Ensure the `<File Id="basilc_exe" ... />` element name matches exactly so the bind works.

6️⃣ Version bump workflow

* The canonical version lives in each crate’s `[package] version` (or at the workspace root).
* For a new release (e.g., `1.1.0`), bump the version(s), rebuild on Windows (so EXEs carry the new FileVersion), then rebuild the MSI — its version automatically follows via binding.

7️⃣ Keep all Windows-specific build logic inside `#[cfg(windows)]` blocks so other platforms remain unaffected.

---

### 🔍 Verification after Junie completes

Run these commands to confirm version embedding and packaging:

```powershell
cargo build -p basilc --release --features obj-all
cargo build -p bcc --release
cargo build -p basil-serve --release
scripts\build-windows-msi.ps1
```

Then right-click any EXE → **Properties → Details** tab → confirm `File version: 1.0.0`.
Install the MSI → check `basilc`, `bcc`, `basil-serve` on PATH and that `Programs and Features` shows version 1.0.0.

---

✅ After these steps, the MSI’s version will always match your compiled EXE version, enabling smooth future upgrades and downgrades.


