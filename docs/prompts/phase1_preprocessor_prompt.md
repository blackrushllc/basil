### Implement Phase 1 Preprocessor with Include Files (Interpreter‑only BASIC)

Context
- Repository: the smaller BASIC project (interpreter only; no bytecode compiler/bcc).
- Goal: Add a compile‑time preprocessor that resolves source directives before lexing/parsing, enabling modular code via include files and simple conditionals.
- Scope: Phase 1 includes #include with include‑once semantics, embedded/library includes, conditional compilation (#define/#undef/#if/#elif/#else/#endif), built‑in predefined names, robust error handling, and CLI flags. Runtime include is out of scope.

High‑level requirements
- Preprocess source to produce a single flattened text plus a SourceMap mapping output positions back to original files/lines for diagnostics.
- Support include‑once and detect cycles with clear messages.
- Support embedded/library includes (files compiled into the executable) via angle‑bracket form.
- Provide CLI options: -I/--include-path (repeatable), --D NAME[=VALUE] (predefine macro), and optionally --no-embedded-includes.
- Keep the core lexer/parser unchanged; they operate on preprocessed text.

Directive syntax and recognition
- A directive is recognized only when # is the first non‑whitespace character on a line (not inside strings).
- Supported directives:
  1) #include forms
     - #include "relative/or/absolute/path.basil"
     - #include <embedded/path.basil>     // search embedded/library first
     - #include path/without/spaces.basil  // bare path; error if spaces (require quotes)
     Semantics: Insert the contents of the resolved file as if pasted at that location. Enforce include‑once globally across the unit.
  2) #define / #undef
     - #define NAME            // a boolean macro (definedness)
     - #define NAME 123        // integer value
     - #define NAME "text"    // string value
     - #undef NAME
  3) Conditionals: #if, #elif, #else, #endif
     - Grammar for expressions (evaluated at preprocess time):
       literals: integers (base 10), strings in double quotes
       identifiers: macro names; boolean true if defined (or its value if scalar)
       operators: ! (unary), == != < <= > >=, && ||
       parentheses: ( ... )
       function: defined(NAME) → 1/0
     - Nesting supported; unmatched or misordered directives are errors.

Built‑in predefined names (read‑only)
- __version__: numeric or semantic version string of the interpreter (choose numeric major or keep full string; support both numeric and string comparisons by type).
- __file__: path of the current file being preprocessed.
- __line__: 1‑based current line number.
- __os__: "windows" | "linux" | "macos" (lowercase).
- __engine__: engine name for this interpreter; e.g., "basic" or the project’s binary name.
- __debug__: 1/0 depending on build profile or a run‑time flag.
Notes: Built‑ins cannot be overridden by #define. If user attempts to redefine, emit a warning or error (choose error for simplicity).

Include resolution and search paths
- Forms and order:
  a) Quoted: #include "foo/bar.basil"
     1) Directory of the current file
     2) Project root (current working directory of the process)
     3) Include paths from CLI -I (left‑to‑right)
     4) Paths from BASIL_PATH environment variable (semicolon‑separated on Windows; colon on Unix)
     5) Embedded/library table (last resort)
  b) Angle‑brackets: #include <lib/eliza.basil>
     1) Embedded/library table (first)
     2) CLI -I paths
     3) Project root
  c) Bare path: treat like quoted form; if it contains spaces, error with hint to use quotes.
- Path handling:
  - Accept both \ and / from users; normalize internally to forward slashes for logical keys.
  - Canonicalize filesystem paths to absolute form; case‑insensitive canonicalization on Windows.
  - Resolve symlinks where available.

Embedded/library includes
- Provide a build‑time generated table of embedded files under includes/ within the repo.
- Build step (build.rs): recursively scan includes/ and generate an embedded_includes.rs that defines:
  struct EmbeddedFile { path: &'static str, contents: &'static [u8] }
  static EMBEDDED_FILES: &[EmbeddedFile]
  where each entry uses include_bytes! on the real file path.
- The preprocessor consults this table when resolving angle‑bracket includes and as a fallback for quoted includes.
- Logical key for embedded files: embedded:/<path> (e.g., embedded:/examples/hello.basil) for uniqueness.

Include‑once semantics and cycle detection
- Maintain a visited set keyed by canonical path or embedded logical key; on second request, silently skip.
- Maintain an include stack (vector of keys). If an include would re‑enter a file already on the active stack, abort with a cycle error that prints the include chain.
- Limits:
  - Max include depth: default 64 (configurable constant).
  - Max expanded size: default e.g., 2 MiB or configurable constant; error if exceeded.

Preprocessor outputs
- PreprocessResult { text: String, source_map: SourceMap, dependencies: Vec<IncludeKey> }
- SourceMap must map output byte offsets (or line/column) back to (file_id, line, column) in original sources for diagnostics.
- File IDs are assigned by a SourceManager maintaining mapping between IDs and either filesystem paths or embedded keys.

Integration points
- Call the preprocessor on the initial source before lexing/parsing/executing.
- Pass SourceMap into the diagnostic renderer so errors from the lexer/parser report correct file/line in included files.
- Add CLI flags to the interpreter binary:
  - -I, --include-path <dir> (repeatable)
  - --D NAME[=VALUE]         (predefine macros)
  - --no-embedded-includes   (optional; disables looking up embedded files)

Rust API sketch (adjust paths/names to your code layout)
- Trait for embedded provider:
  ```rust
  pub trait IncludeProvider { fn try_read(&self, logical: &str) -> Option<&'static [u8]>; }
  ```
- Preprocessor options:
  ```rust
  pub struct PreprocessOptions<'a> {
      pub root_path: std::path::PathBuf,      // the primary file’s path or cwd
      pub include_paths: Vec<std::path::PathBuf>,
      pub env_paths: Vec<std::path::PathBuf>,
      pub embedded: Option<&'a dyn IncludeProvider>,
      pub defines: std::collections::HashMap<String, MacroValue>,
  }
  ```
- Macro values:
  ```rust
  pub enum MacroValue { Bool(bool), Int(i64), Str(String) }
  ```
- Output:
  ```rust
  pub struct PreprocessResult { pub text: String, pub source_map: SourceMap, pub dependencies: Vec<IncludeKey> }
  ```
- Include keys:
  ```rust
  pub enum IncludeKey { Fs(std::path::PathBuf), Embedded(String) }
  ```
- SourceMap: store ranges mapping flattened output spans to (file_id, start_line, start_col), with helpers to translate.

Expression evaluator (for #if)
- Implement a simple recursive‑descent or Pratt parser for precedence: ! > comparisons > && > ||.
- Identifiers resolve as:
  - If defined with value: use that value (ints, strings). Non‑zero int → true; non‑empty string → true.
  - If defined without value: true.
  - If not defined: false.
- Mixed type comparisons:
  - int vs int: numeric compare.
  - string vs string: lexicographic compare; recommend for __version__ if stored as string.
  - other mixes: error.

Error handling and diagnostics
- File not found:
  ```
  error: include not found: "utils/format.basil"
    searched in:
      - C:\\proj\\src
      - C:\\proj
      - -I C:\\basil\\lib
      - BASIL_PATH entries
      - embedded library
    included from: app.basil:12:1
  ```
- Include cycle:
  ```
  error: include cycle detected
    app.basil:3:1 → includes/a.basil:10:1 → includes/b.basil:7:1 → includes/a.basil
  ```
- Depth/size limits:
  ```
  error: maximum include depth (64) exceeded (…include stack…)
  error: preprocessed output exceeds 2 MiB (limit)
  ```
- Conditional parser errors:
  ```
  error: invalid token in #if expression near "foo$"; expected identifier, literal, or (
  error: type mismatch: cannot compare string to int with <
  error: unknown identifier "FEATURE_X" (hint: use defined(FEATURE_X))
  ```
- Directive placement:
  ```
  error: unterminated #if starting at file.basil:42:1 (expected #endif); show include stack context.
  ```
- Redefining built‑ins:
  ```
  error: cannot redefine built‑in macro __version__
  ```

Cross‑platform details
- Normalize line endings (CRLF → LF) for consistent mapping; preserve correct reported columns.
- On Windows, canonicalization is case‑insensitive.

CLI integration
- Parse -I/--include-path into options.include_paths.
- Read BASIL_PATH from env, split by platform delimiter, and push to options.env_paths.
- Parse --D NAME[=VALUE] into defines table: NAME=true if no value; parse int if numeric; otherwise string.

Testing plan
- Unit tests (preprocessor):
  - Single include; nested includes; duplicate includes (ignored) from different locations; cycle detection.
  - Path precedence for quoted vs angle‑bracket.
  - Conditional branches (#if/#elif/#else/#endif): truthy/falsy, defined(), numeric and string compares, nested.
  - Built‑ins availability and stability across files; __file__/__line__ change as lines advance; spans map correctly.
  - Embedded lookup with a small fake provider; dedup across FS and embedded (include‑once by key).
  - Error cases: missing file, limit exceed, malformed expression, redefining built‑ins, unterminated #if.
- Integration tests:
  - End‑to‑end run executing a program using includes and conditionals; confirm output.
  - Verify diagnostics from parser/lexer point to the correct file:line in an included file.

Documentation updates
- Add a Preprocessor Directives page explaining syntax, forms, search paths, include‑once, conditionals, and embedded library usage. Include examples and CLI flags.
- Update keyword lists to note directives are preprocessor constructs.

Acceptance criteria
- Programs can modularize with #include and not re‑include the same file twice.
- Angle‑bracket form locates embedded/library files; quoted form prefers local files.
- Conditional compilation works with built‑ins and CLI‑defined macros.
- Helpful, specific error messages with include stacks and search paths.
- Diagnostics from later stages (lexer/parser/runtime) display the correct original file and line.

Implementation checklist (suggested order)
1) Build the embedded includes table (build.rs → embedded_includes.rs) scanning repo includes/.
2) Add IncludeProvider wired to the generated table.
3) Implement SourceManager and SourceMap.
4) Implement Preprocessor with directives and expression evaluator.
5) Wire preprocessor into interpreter front end; plumb SourceMap into diagnostics.
6) Add CLI flags (-I, --D, --no-embedded-includes) and env BASIL_PATH support.
7) Add limits and graceful errors; add tests; docs.

Out of scope now
- Runtime include() function. Consider later behind a flag and include‑once enforcement.

If anything in your repo differs (names/paths), adapt identifiers accordingly while keeping the behavior exactly as specified above.
