# Building a REPL (Interactive Mode CLI for Basil)

This doc is also an example of my back-and-forth with ChatGPT in developing the concept of a REPL for Basil.

I've removed a large portion of the discussion which preceded this where I reviewed several fundamental concepts of
Basil and how it works, order to make sure that ChatGPT understands the problem and the solution.

> Me: ... So I want to build an "Immediate Mode", or REPL command prompt. I want it to be sort of like old BASICA or GWBASIC where you can
> directly enter commands and see them run, but I also want it to function like Laravel's "artisan tinker" command which
> lets you run in a loaded environment. I'm thinking of adding a new launch directive called "cli" or something. We
> currently have "run", "test", and "lex" which runs basil programs in regular mode, or in test mode which skips over all
> inputs by providing a mock keyboard response and outputs all comments in the code during execution, or lex mode which
> dumps tokens and stuff while the program is running. "cli" mode would load a basil (or basilx) program like normal,
> including build the basilx like it normally would if need be, and the program would execute. It would run through all of
> the module-level code that might be setting variables, dimensioning arrays, or performing whatever basic statements
> might be there, and setting up all of the function definitions, and even loading basilx class objects. The program might
> also just do nothing at all. It doesn't matter. But when the program has finished running, I want a command prompt to
> appear like the way old style BASIC or Laravel Tinker does. Then I want the user to be able to enter statements, or
> multiple statements separated by semicolons (which the language already allows) and then execute them on the fly or
> displaying any errors without terminating the interpreter. I want any runtime or parse errors to simply be reported and
> allow the command prompt entry cycle to resume until the user enters the exit command (which already exists in the
> language). What do you think about this? I'm thinking it might be hard because the Rust project wants to compile and run
> everything as bytecode and it wants to crash at the slightest hint of trouble. How big of a task do you think this might
> be, and I'll follow up after reading your thoughts with a question about building the prompt for Junie Ultimate.

ChatGPT: This is a great fit for a “loaded-environment REPL.” You can keep Basil’s bytecode/VM model and still get an immediate mode by compiling tiny snippet-modules on the fly into the **same live VM** that just finished running the user’s program.

Here’s a concrete plan that keeps things robust and incremental:

# 1) Process model (what “cli” does)

1. **Load & run the target program** exactly as today (`run` path): parse/compile to `.basilx` if needed, execute module-level code, load classes/linked Rust objects, define functions, fill globals, etc.
2. When it returns to the top frame, **don’t tear down the VM**. Drop into a **REPL loop** bound to that same VM + global env + heap.
3. Each REPL entry is compiled as its own **ephemeral module** (a “snippet”) and executed immediately.

Benefits: you get *Tinker*-style access to everything the program set up (globals, DIM’d classes, arrays, loaded objects).

# 2) Snippet compilation strategy

* **Two parse modes**:

    * **Stmt-list mode** (default): parse one or more statements (you already support `;` separators). Good for: `LET`, `DIM`, `FOR…NEXT`, `CALL`, etc.
    * **Expr mode** (quality-of-life): if the line parses as a lone expression, implicitly `PRINT` the result. (Classic immediate-mode feel.)
* **Synthetic wrapper**: compile a snippet as:

    * A tiny module with a single function like `__repl_chunk_N()` containing the parsed stmt-list.
    * Link it against the **current global symbol table** so it can see existing functions, classes, globals.
    * Emit bytecode; **don’t persist** these snippets to disk (or optionally keep a `.basilx` in a temp cache).
* **Name resolution**:

    * Reads of unknown names → resolve against globals or report an error.
    * Writes to unknown names → create/assign **globals** (REPL is top-level).
* **Function/class definitions in REPL**:

    * Optional: allow `FUNC`/`CLASS` definitions in snippets by installing their symbols into globals on successful compile. Later snippets can use them.

# 3) State & safety: run snippets without crashing

* **VM invariants**: Execute each snippet in a fresh call frame. On error, unwind to the REPL top-level; never propagate a panic.
* **Transactional updates**:

    * Parse/compile: if diagnostics exist → report, discard bytecode.
    * Execute: if runtime error/trap occurs → report, **roll back partial side effects** only if you can (see below). Otherwise, document that side effects prior to the fault remain (like Python).
    * Minimal viable path: *don’t* roll back; just guarantee the VM is back at a clean top-of-loop state.
* **Error policy**:

    * Parser/semantic errors → display diagnostics, continue loop.
    * Runtime errors → display error (with stack trace if `:bt` is toggled), continue loop.

# 4) Globals, heap, and lifetimes

* **Globals** live for the session. REPL assigns into the same global frame created by the loaded program.
* **Class instances (handles)** created in REPL persist in the heap; they’re freed when session ends (or when reassigned / GC’d if you have GC; if not, rely on session teardown).
* **Files/IO**: your “auto-close on interpreter exit” still applies; the REPL session is “the interpreter” lifetime.

# 5) Loader integration (source vs .basilx)

* Keep your **ModuleLoader** and **BytecodeCache** around for the session. Snippets link against already-loaded modules.
* When a REPL line references `CLASS("lib/foo.basil")`, use the normal load path: if `.basilx` exists and is fresh, load it; else compile source.
* Consider **`:reload <module>`** to invalidate a module and re-load its `.basil`/`.basilx`.

# 6) UX of the REPL

Add a small command layer (lines starting with `:`):

* `:help` – list commands.
* `:vars` – show globals in scope (optionally filtered).
* `:types` – print type info for names (best-effort).
* `:methods <var>` – reflect available members on a class instance (if you have metadata).
* `:disasm <name>` – disassemble a known function/class method.
* `:reload [module]` – invalidate & reload module(s).
* `:env` – show linked objects/features/features flags.
* `:history`, `:save <file>`, `:load <file>` – usability.
* `:bt on|off` – toggle stack traces on runtime errors.
* `:exit` – end session (or regular `EXIT` already in language).

CLI ergonomics:

* **Multiline input** when block is incomplete (`BEGIN…END`, `IF…END IF`, open quotes, etc.). Keep reading until balanced, then compile.
* **Line editing** with history and completion (e.g., rustyline). Optional completion for globals, function names, `CLASS("...")` paths.
* Optional prompt flair: show current module / last error code, etc.

# 7) Parser/compiler tweaks (usually small)

* Expose a **“parse statements”** entry (not just full modules).
* Allow a **“toplevel”** code path that binds symbols to the current global scope instead of making a brand-new isolated one.
* Make sure diagnostics are **non-panicking** and recoverable (return `Result<Bytecode, Diagnostics>`).
* Ensure the **linker** can link a snippet module that imports from the live program’s symbol table.

# 8) VM execution API

Add a stable API something like:

```rust
pub struct Session {
    vm: Vm,
    globals: GlobalEnv,
    loader: ModuleLoader,
    cache: BytecodeCache,
    history: Vec<String>,
}

impl Session {
    pub fn run_program(&mut self, path: &str) -> Result<(), Diags>;
    pub fn eval_stmtlist(&mut self, src: &str) -> Result<ExecOutcome, Diags>;
    pub fn eval_expr(&mut self, src: &str) -> Result<Value, Diags>;
}
```

Internally, `eval_*`:

1. Lex/parse (stmt-list or expr),
2. Lower to IR/bytecode as module `__repl_chunk_N`,
3. Link against `self.globals`,
4. Push a call frame and `vm.run_until_return()`,
5. Map traps to diagnostics; on success, update `globals` with any new/updated names.

# 9) “cli” entry-point behavior

* `basilc cli` – start empty session (no program).
* `basilc cli main.basil` – load/execute `main.basil` first, then REPL.
* Options:

    * `--no-exec` (only load/define functions/classes, skip module main code),
    * `--features ...` (reuse your feature flags),
    * `--path …` (search path for class files),
    * `--disallow-define` (lockdown mode: statements only).
* Consider `--test-mocks` to reuse your mock input layer in REPL for reproducible demos.

# 10) Complexity & effort (what changes where)

* **Parsing**: add stmt-list/expr entry points; small, contained.
* **Compiler/linker**: support compiling “snippet” modules that link to a pre-existing global env; moderate but straightforward.
* **VM**: ensure recoverable errors + frame cleanup; usually small if you already return `Result` from `run`.
* **Runtime**: a `Session` wrapper that owns VM + loader + cache; moderate glue.
* **CLI**: command loop + line editor + helpers; small-to-moderate.
* **Nice-to-haves** (optional later): symbol reflection, disassembler view, completion metadata.

Overall: **very doable** without undermining your bytecode design. The key is to **preserve the VM and globals between snippet executions** and to make compilation/execution return diagnostics rather than abort. You don’t need hot patching of existing bytecode—just keep linking new snippet modules into the same session and let them call/modify global state.

If this direction feels right, I can draft the Junie prompt that asks her to:

* add the `cli` subcommand,
* implement `Session`/snippet compilation,
* wire REPL grammar (stmt-list + expr),
* add the `:` commands above,
* and define the exact error-handling contract.

> Me: I love it! We don't need the test mocks and we don't need to worry about reloading class files, I don't think that
> functionaty is necessary and might be difficult to manage internally. I LOVE the multi-line support. I think it might be
> touch for Basil to really determine when a block ends in some cases, so an alternative to trusting the parser to know
> when a block is actually done, how about we require a double semicolon to execute the code IN ALL CASES? I know this
> sounds kind of funny, but not only would this prevent someone from prematurely executing code that they didn't mean to,
> it would also make it really easy to enter many commands on multiple lines at one time before letting them get run.
> Finally, I'd like to alias just the word "quit" with ":exit" because that's common, and I'd also like to alias the
> word "system" with ":exit" as a nod to old school basic. I would like users to be able to exit shell commands with !,
> for instance "!dir *.basil" or "!pwd" or even "!bash" to run the command shell (if on unix/linux) and then exit back to
> the Basil CLI. I also love the :vars and :methods and similar commands because that would really make debugging with the
> CLI even more powerful. Can you make me one of your cool Junie prompts now?

# Junie Prompt — Implement Basil “cli” Immediate Mode / REPL

You are helping build **Basil**, a modern BASIC interpreter written in Rust. Basil compiles `.basil` sources to `.basilx` bytecode on first execution or code change. Programs can load **class files** (regular Basil files whose main body serves as a constructor and whose LETs are public properties, with FUNC methods). Classes are instantiated like:

```
DIM user@ AS CLASS("my_class.basil");
```

Basil supports running in different modes (e.g., `run`, `test`, `lex`). We want a new **`cli`** mode that behaves like an **Immediate Mode REPL** (think GW-BASIC/BASICA) but over the **already loaded** environment of a program (similar to Laravel Tinker): we first run a program (optional), then drop into a REPL that can evaluate further statements/expressions against the same live VM, globals, classes, and linked objects.

## High-Level Goals

* Add a `cli` subcommand that:

    1. Optionally loads and executes a Basil program (compiling to `.basilx` as needed).
    2. Keeps the **same VM + global environment** alive after program completion.
    3. Starts a **REPL** that accepts user input and evaluates it **in that environment**.
* **Execution trigger is a double semicolon `;;`**. Input may span multiple lines; nothing is compiled/executed until `;;` is encountered.
* The REPL must handle **statements** (stmt-list) and **expressions** (if a single expression, implicitly `PRINT` the result).
* Add a small set of REPL “meta commands” starting with `:` (listed below).
* Support **shell escapes** with `!` (e.g., `!pwd`, `!dir *.basil`, `!bash` to spawn a shell and return).
* Provide **aliases**: the plain words `quit` and `system` both behave like `:exit`.

## Non-Goals / Out of Scope (for this task)

* No “test mocks” integration in `cli`.
* No class/module **reload** feature.
* No background tasks or file watchers.
* No persistent saving of snippet bytecode to disk (in-memory is fine).

## User Experience Specification

### Launch

* `basilc cli` — start a REPL with an **empty** global environment (same feature flags as default).
* `basilc cli path/to/main.basil` — load & execute `main.basil` first (building/loading `.basilx`), then drop into REPL with that environment.
* Support the usual `--features ...` flags (same semantics as other modes).

### Input & Execution

* **Multiline by default**. The REPL buffers input lines until it sees `;;` at the end of a line (whitespace allowed after). Only then:

    * Strip the trailing `;;`,
    * Treat the buffered text as a **single snippet**,
    * Compile + execute it against the live session.
* If the snippet parses as **a single expression**, implicitly print its value:

    * e.g., `1 + 2 * 3 ;;` → prints `7`.
* If the snippet parses as **statements**, just execute them. (Multiple statements separated by single `;` are supported inside the snippet.)
* On **parse/semantic error**: show diagnostics, do **not** exit the REPL, discard the snippet, continue.
* On **runtime error**: print the error (and optional backtrace if enabled), **do not** terminate the session; return to prompt.

### Meta Commands (lines beginning with `:`)

Implement (exact spelling):

* `:help` — brief summary of meta commands.
* `:vars [filter]` — list global variables (name, brief type/value preview). Optional substring `filter`.
* `:types [name]` — if `name` given, show type info; otherwise summarize known user types/classes/functions.
* `:methods <var>` — reflect/print public methods & properties available on a class instance bound to `<var>`. (Best-effort if limited metadata.)
* `:disasm <name>` — disassemble a known function/class method by symbol name.
* `:history` — show recent snippet starts (first line) or a numbered list to help re-run.
* `:save <file>` — write the **current buffered (not yet executed)** snippet to `<file>`; if no buffered snippet, write the last executed snippet.
* `:load <file>` — read file contents as a snippet (append to buffer), do not execute until `;;` provided (user can add or just enter `;;`).
* `:bt on|off` — toggle runtime backtraces on errors.
* `:env` — show feature flags, search paths, and basic VM info.
* `:exit` — exit the REPL.

**Aliases**:

* A bare `quit` → same as `:exit`.
* A bare `system` → same as `:exit`.

### Shell Escapes

* Any line starting with `!` runs a **system shell** command **immediately** (does not use the `;;` rule). Examples:

    * `!pwd`
    * `!dir *.basil`
    * `!bash` (spawn interactive shell; when the shell exits, return to the REPL)
* Print the child’s stdout/stderr pass-through. Return to prompt.
* **Security**: this is a developer tool; document that shell escapes execute commands as the current user.

### Prompt & Editing

* Use a line-editing crate (e.g., `rustyline`) with history.
* Prompt text:

    * Primary prompt: `basil> `
    * Continuation prompt (while buffering before `;;`): `...> `
* Tab completion (optional if simple): propose meta commands, `:` commands, and shell `!` prefix; future-proof hooks for symbol completion.

## Compiler/VM Integration Requirements

We already have: lex/parse → lower → bytecode → VM execution. Preserve that design; the REPL compiles **ephemeral “snippet modules”** that link against the session’s **current global environment**.

### Data Structures & APIs (target shape)

Create a `Session` owning the live VM + loader + cache:

```rust
pub struct Session {
    vm: Vm,
    globals: GlobalEnv,        // shared/global symbol frame for the session
    loader: ModuleLoader,      // can load basil/basilx modules
    cache: BytecodeCache,      // uses normal basilx path rules
    next_snippet_id: usize,    // __repl_chunk_{N}
    settings: SessionSettings, // { show_backtraces: bool, features: ..., search_paths: ... }
    history: Vec<String>,      // raw snippet texts (for :history / :save)
}

impl Session {
    pub fn new(settings: SessionSettings) -> Self;
    pub fn run_program(&mut self, path: &str) -> Result<(), Diags>;

    /// Compile/eval a snippet that may be a stmt-list or a single expression.
    /// If expression-only and expr_eval==true, print/return its value.
    pub fn eval_snippet(&mut self, src: &str) -> Result<EvalOutcome, Diags>;

    /// Utility for meta commands:
    pub fn list_globals(&self, filter: Option<&str>) -> Vec<GlobalEntry>;
    pub fn type_of(&self, name: &str) -> Option<TypeInfo>;
    pub fn methods_of(&self, var: &str) -> Option<Vec<MethodSig>>;
    pub fn disasm(&self, symbol: &str) -> Result<String, Diags>;
}
```

**Eval flow**:

1. Build a temporary module name `__repl_chunk_{N}`.
2. Try **expression parse first** (fast path). If success and it’s a lone expression, lower to a small function that evaluates and returns the value; REPL prints it.
3. Else **statement-list parse** into a function body (no implicit `PRINT`).
4. Link the snippet against the **current session globals** (functions/classes/globals from the loaded program and previous snippets).
5. Execute in a fresh call frame; capture result/errors. On success, **persist any new/updated globals** (assignments define or update names in the global frame).
6. Do **not** panic; all errors return as diagnostics.

**Globals semantics**:

* Reading an undefined name → diagnostic.
* Assigning to a new name at top level defines a **global**.
* Creating class instances in snippets persists them for the session lifetime (freed on session exit).

**Bytecode cache**:

* Normal program/class modules use the existing `.basilx` behavior.
* **Snippets** are **in-memory only** (no `.basilx` written). It’s OK to store their bytecode objects inside the session.

## Parser/Lowering Requirements

* Provide public entry points for:

    * **Expression-only parse** (returns AST expr if the entire snippet is a single expression).
    * **Statement-list parse** (returns a Vec<Stmt> from arbitrary BASIC statements).
* Allow declarations (`LET`, `DIM`, `FUNC`, `CLASS`) at “toplevel” snippet context:

    * Installing new `FUNC`/`CLASS` symbols into the session’s global env on successful compilation.
    * For `CLASS("file")` references: use the normal loader (source→basilx compile if needed).
* Ensure diagnostics are recoverable and **never abort** the process.

## CLI Wiring

Add a `cli` subcommand to `basilc`:

```
basilc cli [--features ...] [path/to/program.basil]
```

Behavior:

* Initialize `Session` with features/search paths consistent with `run`.
* If a path is provided:

    * Call `session.run_program(path)`.
    * On error, print diagnostics and **still** start REPL (unless fatal linker init failure).
* Enter REPL loop:

    * Read lines, append to a buffer.
    * If a line starts with `:` → handle meta command immediately (buffer unaffected).
    * If a line starts with `!` → run shell escape immediately (buffer unaffected).
    * Otherwise, accumulate lines until `;;` encountered → `session.eval_snippet(buffer_without_trailing_;;)`, then clear buffer.
* Handle `quit` and `system` as synonyms for `:exit`.

## Meta Command Details

* `:vars [filter]`:

    * Show entries like `name : Type = preview`. Preview: brief for numbers/strings; `<class MY_CLASS @0x…>` for objects.
* `:types [name]`:

    * `:types` with no arg → summary counts of functions/classes and a few sample names.
    * `:types name` → print resolved type, or “not found”.
* `:methods <var>`:

    * If `<var>` bound to a class instance, list its public properties + methods (names and arity). If no metadata, list names exposed by your runtime vtable.
* `:disasm <name>`:

    * Disassemble function/method bytecode to a string and print it.
* `:history`:

    * Show last N snippets with index numbers; store at least 100 by default.
* `:save <file>` / `:load <file>`:

    * Save/load raw snippet text. `:load` appends to buffer; user hits `;;` to run.
* `:bt on|off`:

    * Toggle printing backtraces on runtime errors.
* `:env`:

    * Print features, search paths, and any VM/version info.
* `:exit`:

    * Clean shutdown with resource finalization (auto-close files, etc.).

## Error Handling & Stability

* All compile/link/exec paths return `Result<_, Diags>`; **no panics**.
* After any error, the REPL must remain responsive and globals must remain consistent with any completed side effects up to the point of failure (no transactional rollback required).
* Ensure each snippet runs in a fresh call frame; after execution (success or failure), the VM is at a clean REPL top state.

## Minimal Tests / Demos to Include

1. **Expr print**
   Input:

   ```
   1 + 2 * 3 ;;
   ```

   Output contains `7`.

2. **Stmt list**

   ```
   LET A% = 10; LET B% = 5; PRINT A% + B% ;;
   ```

   Output contains `15`.

3. **Multiline until `;;`**

   ```
   LET S$ = "Hello"
   PRINT S$ + ", World" ;;
   ```

   Output contains `Hello, World`.

4. **Use program environment** (when launched with `main.basil`)

    * If `main.basil` sets `LET GREET$ = "Hi"`, then:

   ```
   PRINT GREET$ + " from REPL" ;;
   ```

   Output contains `Hi from REPL`.

5. **Class instance**

   ```
   DIM u@ AS CLASS("classes/my_class.basil"); CALL u@.Init("Erik"); PRINT u@.Greeting$(); ;;
   ```

   Output contains greeting with “Erik”.

6. **Meta commands**

    * `:vars`, `:types u@`, `:methods u@`, `:disasm Greeting$`.

7. **Shell escapes**

    * `!pwd` prints a path.
    * `!bash` starts a shell; `exit` returns to `basil>`.

8. **Errors don’t exit**

    * `PRINT X_DOES_NOT_EXIST ;;` → diagnostic printed, prompt returns.

## Deliverables

* New `cli` subcommand implementation with REPL loop.
* `Session` type and APIs as above (or closely equivalent).
* Parser entry points for **expression** and **statement-list** snippets.
* Bytecode linker support for snippet modules that resolve against the live global env.
* Meta commands + shell escape handling.
* Docs: `docs/repl.md` with usage, `;;` rule, meta commands, and examples.

**Notes & Constraints to Respect**

* The **`;;` execution rule is mandatory** for all REPL code execution (prevents accidental premature runs and simplifies multiline handling).
* Keep snippet bytecode **in-memory**; do not write `.basilx` for snippets.
* No reloader / test mocks in this task.
* Maintain the existing behavior that files auto-close when the interpreter exits.

---

That’s the full spec. Please implement the `cli` mode and its REPL per the above, including the exact `;;` trigger, meta commands, shell escapes, and error-resilient execution in the live environment.
