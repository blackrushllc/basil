To add support for accessing CLI parameters in the Basil BASIC language, we need to modify the VM, the compiler, and the CLI entry point (`basilc`). This will provide the `COMMAND$` keyword, `ARGC%`, `ARGV$`, and helper functions for switches.

Since I am in readonly mode, I have prepared the implementation details below for you to apply to the codebase.

### 1. VM Changes (`basilcore/vm/src/lib.rs`)

We need to store the CLI arguments in the `VM` struct and implement the logic for the new built-in functions.

#### Struct Modification
Add the `cli_args` field to the `VM` struct:
```rust
pub struct VM {
    // ... existing fields ...
    cli_args: Vec<String>,
}
```

#### Initialization
Initialize `cli_args` in `VM::new`:
```rust
impl VM {
    pub fn new(p: BCProgram) -> Self {
        // ...
        let mut s = Self {
            // ... existing initializers ...
            cli_args: Vec::new(),
            // ...
        };
        // ...
        s
    }
}
```

#### New Method
Add a method to populate the arguments:
```rust
impl VM {
    pub fn set_cli_args(&mut self, args: Vec<String>) {
        self.cli_args = args;
    }
}
```

#### Built-in Logic
In the `Op::Builtin` match block (around line 1827), add cases for the new IDs:
```rust
                        160 => { // COMMAND$()
                            if argc != 0 { return Err(BasilError("COMMAND$ expects 0 arguments".into())); }
                            self.stack.push(Value::Str(self.cli_args.join(" ")));
                        }
                        161 => { // ARGC%() or ARGC()
                            if argc != 0 { return Err(BasilError("ARGC% expects 0 arguments".into())); }
                            self.stack.push(Value::Int(self.cli_args.len() as i64));
                        }
                        162 | 163 => { // ARGV$(index%) or PARAM$(index%)
                            if argc == 0 {
                                // Return the whole array
                                let arr = VM::make_string_array(self.cli_args.clone());
                                self.stack.push(arr);
                            } else if argc == 1 {
                                let idx = self.to_i64(&args[0])?;
                                // 1-based indexing for BASIC
                                if idx < 1 || idx > self.cli_args.len() as i64 {
                                    self.stack.push(Value::Str("".into()));
                                } else {
                                    self.stack.push(Value::Str(self.cli_args[(idx - 1) as usize].clone()));
                                }
                            } else {
                                return Err(BasilError("ARGV$ expects 0 or 1 argument".into()));
                            }
                        }
                        164 => { // HAS_SWITCH%(name$)
                            if argc != 1 { return Err(BasilError("HAS_SWITCH% expects 1 argument".into())); }
                            let name = match &args[0] { Value::Str(s)=>s.clone(), _=> return Err(BasilError("HAS_SWITCH% arg must be string".into())) };
                            let found = self.cli_args.iter().any(|a| a == &name);
                            self.stack.push(Value::Bool(found));
                        }
                        165 => { // GET_SWITCH$(name$)
                            if argc != 1 { return Err(BasilError("GET_SWITCH$ expects 1 argument".into())); }
                            let name = match &args[0] { Value::Str(s)=>s.clone(), _=> return Err(BasilError("GET_SWITCH$ arg must be string".into())) };
                            if let Some(pos) = self.cli_args.iter().position(|a| a == &name) {
                                let mut result = Vec::new();
                                for i in (pos + 1)..self.cli_args.len() {
                                    // Stop if we hit another switch
                                    if self.cli_args[i].starts_with('-') { break; }
                                    result.push(self.cli_args[i].clone());
                                }
                                self.stack.push(Value::Str(result.join(" ")));
                            } else {
                                self.stack.push(Value::Str("".into()));
                            }
                        }
```

### 2. Compiler Changes (`basilcore/compiler/src/lib.rs`)

We need to register the new keywords and map them to the IDs used in the VM.

#### Handle Keyword usage in `Expr::Var`
In `emit_expr_in` under `Expr::Var(name)` (around line 1720), add support for zero-parameter calls without parentheses:
```rust
            Expr::Var(name) => {
                let uname = name.to_ascii_uppercase();
                if uname == "COMMAND$" {
                    chunk.push_op(Op::Builtin); chunk.push_u8(160u8); chunk.push_u8(0u8);
                    return Ok(());
                }
                if uname == "ARGC" || uname == "ARGC%" {
                    chunk.push_op(Op::Builtin); chunk.push_u8(161u8); chunk.push_u8(0u8);
                    return Ok(());
                }
                if uname == "ARGV$" || uname == "PARAM$" {
                    chunk.push_op(Op::Builtin); chunk.push_u8(162u8); chunk.push_u8(0u8);
                    return Ok(());
                }
                // ... existing PRO / NOT_PRO checks ...
```

#### Handle Keyword usage in `Expr::Call`
In `emit_expr_in` under `Expr::Call { callee, args }` (inside the `bid` match, around line 1897):
```rust
                    let bid = match &*uname {
                        // ... existing mappings ...
                        "COMMAND$" => Some(160u8),
                        "ARGC"     => Some(161u8),
                        "ARGC%"    => Some(161u8),
                        "ARGV$"    => Some(162u8),
                        "PARAM$"   => Some(163u8),
                        "HAS_SWITCH"  => Some(164u8),
                        "HAS_SWITCH%" => Some(164u8),
                        "GET_SWITCH$" => Some(165u8),
                        // ...
```

### 3. CLI Changes (`basilc/src/main.rs`)

We need to capture the remaining arguments from the command line and pass them to the VM.

#### Update `cmd_run`
Change the signature of `cmd_run` and pass the arguments to the VM:
```rust
fn cmd_run(path: Option<String>, cli_args: Vec<String>) {
    // ... existing loading and compilation logic ...

    // Run VM
    let mut vm = VM::new(program);
    vm.set_cli_args(cli_args); // Pass the captured arguments
    vm.set_script_path(abs_path.to_string_lossy().to_string());
    // ...
}
```

#### Update `cli_main` call to `cmd_run`
In `cli_main` (around line 635):
```rust
        "run" => {
            let path = args.get(0).cloned();
            let mut script_args = Vec::new();
            if args.len() > 1 {
                script_args = args[1..].to_vec();
            }
            cmd_run(path, script_args);
        }
```

#### Update `cmd_test`
Similarly, update `cmd_test` to collect any "unknown" arguments and pass them to the VM as `cli_args`.

### Summary of New Built-ins
- `COMMAND$`: Returns the full command line string (joined by spaces).
- `ARGC%` / `ARGC`: Returns the number of command line arguments.
- `ARGV$()`: Returns an array of all arguments.
- `ARGV$(i%)` / `PARAM$(i%)`: Returns the i-th argument (1-based index).
- `HAS_SWITCH%(name$)`: Returns true if the switch (e.g., `--verbose`) exists.
- `GET_SWITCH$(name$)`: Returns the parameters following a switch until the next switch or end of arguments.

This implementation covers both the classic `COMMAND$` approach and modern `argc/argv` styles, while providing powerful helpers for CLI application developers.

### Recent Changes (Error Reporting & Code Quality)

#### 1. Enhanced Error Reporting
- Implemented a source mapping mechanism that translates internal line numbers to original source file names and line numbers for all compile and parse errors.
- Added a `basil_error` helper in `basilcore/common` to standardize error formatting as `at line N in filename: message`.
- Modified the `basilc` CLI to catch and format errors using the `SourceMapMini` before displaying them to the user.
- Updated the "SUB call has no value" compile error to explicitly include the name of the SUB being called and the correct location in the source file.

#### 2. Code Quality & Consistency
- Resolved `dead_code` warnings in `basilcore/parser` and `basilcore/lexer` by integrating the `self.error()` helper method.
- Standardized error reporting across the lexer and parser to ensure consistent use of line-number prefixes.
- Improved error messages for various edge cases (e.g., CONST name type suffixes, string interpolation errors).