The `/scripts/` directory contains various PowerShell and Bash scripts used for building, packaging, and maintaining the
Basil project. Below is an explanation of the purpose and function of each file:

### Build and Distribution Scripts

#### Windows

* `build_dist_windows.ps1`: The primary build script for Windows. It compiles multiple variants of the `basilc` and
  `bcc` binaries using different Cargo feature flags (e.g., `naked`, `bmx`, `daw`, `all`, `web`) and organizes them into
  the `dist\windows` directory.
* `build_dist_windows`: A PowerShell wrapper script that invokes `build_dist_windows.ps1`.
* `build-windows-msi.ps1`: Automates the creation of a Windows installer (`.msi`). It builds the project with all
  features, stages documentation and examples (applying specific exclusion rules), and then uses the WiX Toolset to
  compile the installer.
* `build-windows.ps1`: Similar to `build-windows-msi.ps1`, but it skips the compilation step (assuming binaries are
  already built) and proceeds directly to staging files and building the MSI installer.

#### Linux

* `build_dist_linux.sh`: The primary Bash build script for Linux. Like its Windows counterpart, it builds various
  variants of `basilc` and `bcc` (e.g., `naked`, `all`, `bmx`, `daw`, `web`) and copies them to `dist/linux`.
* `build_dist_linux`: A Bash wrapper script that invokes `build_dist_linux.sh`.

#### macOS

* `build_dist_mac.sh`: The build script for macOS. It follows the same logic as the Linux and Windows scripts, building
  specific variants and placing them in `dist/mac`.

### Code Maintenance and Transformation Scripts

* `lowercase_basil_keywords.ps1`: A sophisticated utility that converts Basil language keywords to lowercase in `.basil`
  files. It includes logic to ignore keywords found inside strings or comments (both `REM` and `'` styles).
* `do_replace.ps1`: A general-purpose utility for performing whole-word token replacements across all `.basil` files in
  a directory (defaults to `examples`).
* `quick_token_pass.ps1`: A simplified version of the replacement logic that specifically targets certain keywords (like
  `FUNC`, `DIM`, `EXIT`, `EACH`) to ensure they are lowercase across the example files.

### Testing and Development Scripts

* `run_basic_ide_step1_test.ps1`: A script designed to test the IDE integration features. It performs three main tasks:
    1. Runs the Rust `debug_api` tests in the `basil-vm` crate.
    2. Runs `basilc` in analysis mode (`--analyze`) on a demo file.
    3. Runs `basilc` in debug mode (`--debug`) to verify that JSON debug events are correctly generated.