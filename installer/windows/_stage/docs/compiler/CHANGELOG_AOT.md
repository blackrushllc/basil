# AOT (bcc) recent changes

- Move final executable into the invoking directory (CWD) and name it after the source file (hello.basil → hello.exe on Windows, hello on Unix). Overwrites if present and prints the local path.
- Generated Cargo projects now include an empty [workspace] table to isolate them from ancestor workspaces (prevents `current package believes it's in a workspace` errors when building from within the Basil repo).
- Local runtime path handling improved: `--dep-source local --local-runtime <path>` now canonicalizes and sanitizes Windows paths (strips `\\?\` prefixes, uses forward slashes).
