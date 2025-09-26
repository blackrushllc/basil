# Basil 🌿
A modern BASIC‑flavored language focused on web/back‑end.

# 🍿 HERE'S THE LATEST UPDATE :

# Basil (prototype v0)

Minimal public prototype: CLI + lexer that prints tokens.

Build:

```bash
cargo run -p basilc -- examples/hello.basil
````
Quick Try:

````# tokenize an example file
cargo run -p basilc -- run examples/hello.basil


# or with the punny alias
cargo run -p basilc -- sprout examples/hello.basil
````

# init a new project in ./myapp (or use alias `seed`)
````cargo run -p basilc -- init myapp````

````cargo run -p basilc -- seed myapp````

#### 🐷 That's All Folks ! ! ! 

PS. ChatGPT says that "Basil" is a "Pun Farm".  


# 🚀 HERE IS THE PLAN:



# Basil Prototype v0 — Public Plan & Skeleton

A minimal, public‑ready blueprint for a modern BASIC‑flavored language focused on web/back‑end. This plan targets a tiny, end‑to‑end slice: **source → tokens → AST → bytecode → VM** with room to evolve into C/WASM/JS backends.

---

## 0) High‑level goals

* **Developer joy**: BASIC warmth + modern features (expressions, async later, modules).
* **Simple core now, room to grow**: start with a stack VM, evolve to register/SSA.
* **Interop first**: design a stable C ABI and WASI component boundary (later phases).
* **Linux + Windows, single binary toolchain**.

---

## 1) Repository layout (Rust host)

```
basil/
├─ LICENSE
├─ README.md
├─ Cargo.toml                    # workspace
├─ basilc/                       # CLI (repl, run, compile)
│  ├─ Cargo.toml
│  └─ src/main.rs
├─ basilcore/                    # language core crates
│  ├─ lexer/         (tokens + scanner)
│  ├─ parser/        (Pratt parser → AST)
│  ├─ ast/           (AST nodes + spans)
│  ├─ compiler/      (AST → bytecode chunk)
│  ├─ bytecode/      (opcodes, chunk, constants)
│  ├─ vm/            (stack VM, values, GC stub)
│  └─ common/        (errors, interner, span, arena)
├─ stdlib/                       # native builtins (print, clock) and later modules
├─ examples/
│  ├─ hello.basil
│  ├─ expr.basil
│  └─ fib.basil
└─ tests/
   └─ e2e.rs
```

> Later: `emit_c/`, `emit_wasm/`, `bridge_napi/`, `bridge_hpy/`, `ffi_c_abi/`.

---

## 2) Language subset v0 (EBNF)

```
program     := { declaration } EOF ;

declaration := "FUNC" ident "(" [parameters] ")" [":" type] block
             | "LET" ident [":" type] "=" expression ";"
             | statement ;

parameters  := ident [":" type] { "," ident [":" type] } ;

statement   := expr_stmt
             | if_stmt
             | while_stmt
             | return_stmt
             | block ;

block       := "BEGIN" { declaration } "END" ;      // BASIC-y but modernized

expr_stmt   := expression ";" ;
if_stmt     := "IF" expression "THEN" statement [ "ELSE" statement ] ;
while_stmt  := "WHILE" expression "DO" statement ;
return_stmt := "RETURN" [ expression ] ";" ;

expression  := assignment ;
assignment  := IDENT "=" assignment | logic_or ;
logic_or    := logic_and { "OR" logic_and } ;
logic_and   := equality  { "AND" equality } ;
equality    := comparison { ("==" | "!=") comparison } ;
comparison  := term      { ("<" | "<=" | ">" | ">=") term } ;
term        := factor    { ("+" | "-") factor } ;
factor      := unary     { ("*" | "/") unary } ;
unary       := ("NOT" | "-" | "+") unary | call ;
call        := primary { "(" [ arguments ] ")" } ;
primary     := NUMBER | STRING | TRUE | FALSE | NULL | IDENT | "(" expression ")" ;

arguments   := expression { "," expression } ;

type        := IDENT ; // placeholder for v0, optional annotations only
```

---

## 3) Tokens (v0)

```
Enum TokenKind {
  // single char
  LParen, RParen, Comma, Semicolon,
  Plus, Minus, Star, Slash,
  Lt, Gt,
  // one or two char
  Assign, EqEq, BangEq, LtEq, GtEq,
  // literals/identifiers
  Ident, Number, String,
  // keywords
  Func, Return, If, Then, Else, While, Do, Begin, End,
  Let, True, False, Null, And, Or, Not,
  Eof,
}
```

* Scanner produces `(kind, lexeme_span, literal?)` with line/column spans.

---

## 4) Pratt parser outline (binding powers)

Binding power table (lowest → highest):

```
=          : 10   (right-assoc, handled specially on IDENT)
OR         : 20
AND        : 30
== !=      : 40
< <= > >=  : 50
+ -        : 60
* /        : 70
prefix     : 80   (NOT, unary -, unary +)
call()     : 90   (postfix)
primary    : 100
```

**Core Pratt loop** (pseudo‑Rust):

```rust
fn parse_bp(&mut self, min_bp: u8) -> Expr {
    let mut lhs = self.parse_prefix()?; // nud
    loop {
        let op = self.peek();
        let (lbp, rbp) = infix_binding_power(op)?; // led
        if lbp < min_bp { break; }
        self.bump();
        let rhs = self.parse_bp(rbp)?;
        lhs = Expr::Binary { op, lhs: box lhs, rhs: box rhs };
    }
    Ok(lhs)
}
```

* Assignment is a special case: if `lhs` is an `Expr::Var(name)` and next is `=`, parse RHS with right‑binding power of 9.

---

## 5) AST (v0)

```rust
enum Stmt {
  Let { name: IdentId, init: Expr },
  Expr(Expr),
  If { cond: Expr, then_branch: Box<Stmt>, else_branch: Option<Box<Stmt>> },
  While { cond: Expr, body: Box<Stmt> },
  Return(Option<Expr>),
  Block(Vec<Stmt>),
  Func { name: IdentId, params: Vec<IdentId>, body: Vec<Stmt> },
}

enum Expr {
  Literal(ValueLit),              // Number(f64), String(InternId), Bool, Null
  Var(IdentId),
  Assign { name: IdentId, value: Box<Expr> },
  Unary { op: TokenKind, rhs: Box<Expr> },
  Binary { op: TokenKind, lhs: Box<Expr>, rhs: Box<Expr> },
  Call { callee: Box<Expr>, args: Vec<Expr> },
}
```

---

## 6) Bytecode format v0 (stack‑based)

**Why stack first?**

* Easiest to emit from AST.
* Minimal VM loop; great for bootstrapping.
* We can later add a register/SSA IR and keep this as a portable baseline.

**Chunk layout**

```
Chunk {
  code: Vec<u8>,           // bytecodes & operands
  lines: Vec<u32>,         // optional for errors
  consts: Vec<Value>,      // constants pool
}
```

**Instruction encoding**

* 1‑byte opcode + inline operands (u8/u16 as needed, little endian).
* Jumps use u16 offsets (relative forward/back).

**Initial opcodes**

```
// stack effects in comments
CONST_U8   cidx         // push consts[cidx]                 [+1]
POP                      // pop top                           [-1]
DUP                      // duplicate top                     [+1]

LOAD_LOCAL idx           // push locals[idx]                  [+1]
STORE_LOCAL idx          // locals[idx] = pop()               [-1]
LOAD_GLOBAL gidx         // [+1]
STORE_GLOBAL gidx        // [-1]

ADD SUB MUL DIV          // binary numeric ops                [-1]
NEG                      // unary -                           [0]
NOT                      // logical not                       [0]
EQ NE LT LE GT GE        // comparisons → bool                [-1]

JUMP offset              // ip += offset                      [0]
JUMP_IF_FALSE offset     // if !truthy(pop) jump              [-1]

CALL argc                // call value(fn, argc args)         [-argc]
RET                      // return from current frame         [*]
PRINT                    // debug print top (pop)             [-1]
HALT
```

---

## 7) Values & stack frames

```rust
enum Value {
  Null,
  Bool(bool),
  Num(f64),
  Str(InternId),
  Func(FuncObjId),
  Native(NativeFnId),
}

struct CallFrame {
  func: FuncObjId,
  ip: usize,         // instruction pointer into chunk
  base: usize,       // stack base for locals
}

struct VM {
  stack: Vec<Value>,
  frames: Vec<CallFrame>,
  globals: Vec<Value>,
}
```

> GC: v0 uses `Vec` + reference‑counted function objects; later replace with precise GC.

---

## 8) Minimal VM loop (Rust)

```rust
fn run(&mut self) -> Result<(), VMError> {
    use Op::*;
    loop {
        let op = self.read_op();
        match op {
            CONST_U8 => {
                let idx = self.read_u8() as usize;
                let v = self.chunk.consts[idx].clone();
                self.stack.push(v);
            }
            POP => { self.stack.pop(); }

            LOAD_LOCAL => {
                let i = self.read_u8() as usize;
                let base = self.cur().base;
                let v = self.stack[base + i].clone();
                self.stack.push(v);
            }
            STORE_LOCAL => {
                let i = self.read_u8() as usize;
                let v = self.pop();
                let base = self.cur().base;
                self.stack[base + i] = v;
            }

            ADD => bin_num(self, |a,b| a+b)?,
            SUB => bin_num(self, |a,b| a-b)?,
            MUL => bin_num(self, |a,b| a*b)?,
            DIV => bin_num(self, |a,b| a/b)?,
            NEG => { let a = as_num(self.pop())?; self.stack.push(Value::Num(-a)); }

            EQ => bin_cmp(self, |a,b| a==b)?,
            NE => bin_cmp(self, |a,b| a!=b)?,
            LT => bin_num_cmp(self, |a,b| a<b)?,
            LE => bin_num_cmp(self, |a,b| a<=b)?,
            GT => bin_num_cmp(self, |a,b| a>b)?,
            GE => bin_num_cmp(self, |a,b| a>=b)?,

            NOT => { let t = is_truthy(&self.pop()); self.stack.push(Value::Bool(!t)); }

            JUMP => { let off = self.read_u16(); self.ip += off as usize; }
            JUMP_IF_FALSE => {
                let off = self.read_u16();
                let cond = !is_truthy(&self.pop());
                if cond { self.ip += off as usize; }
            }

            CALL => {
                let argc = self.read_u8() as usize;
                self.call(argc)?; // resolves Native or Func, sets new frame
            }
            RET => {
                if !self.ret()? { return Ok(()); } // false -> returned from top
            }
            PRINT => { println!("{:?}", self.pop()); }
            HALT => return Ok(()),
        }
    }
}
```

Helpers (sketch):

```rust
fn bin_num<F: Fn(f64,f64)->f64>(vm: &mut VM, f: F) -> Result<(), VMError> {
    let b = as_num(vm.pop())?; let a = as_num(vm.pop())?;
    vm.stack.push(Value::Num(f(a,b))); Ok(())
}
fn bin_cmp<F: Fn(&Value,&Value)->bool>(vm: &mut VM, f: F) -> Result<(), VMError> {
    let b = vm.pop(); let a = vm.pop();
    vm.stack.push(Value::Bool(f(&a,&b))); Ok(())
}
```

---

## 9) Compiler (AST → bytecode) — essentials

### 9.1 Expression emission

```rust
fn emit_expr(&mut self, e: &Expr) {
  match e {
    Expr::Literal(v) => {
      let idx = self.add_const(v.clone().into_value());
      self.emit(Op::CONST_U8);
      self.emit_u8(idx as u8);
    }
    Expr::Var(id) => {
      let slot = self.resolve_local(*id); // or global
      self.emit(Op::LOAD_LOCAL);
      self.emit_u8(slot);
    }
    Expr::Assign { name, value } => {
      let slot = self.resolve_local(*name);
      self.emit_expr(value);
      self.emit(Op::STORE_LOCAL);
      self.emit_u8(slot);
      self.emit(Op::LOAD_LOCAL); // leave value on stack as expression result
      self.emit_u8(slot);
    }
    Expr::Unary { op, rhs } => { self.emit_expr(rhs); match op { TokenKind::Minus => self.emit(Op::NEG), TokenKind::Not => self.emit(Op::NOT), _ => unreachable!() } }
    Expr::Binary { op, lhs, rhs } => {
      self.emit_expr(lhs); self.emit_expr(rhs);
      self.emit(match op { TokenKind::Plus=>Op::ADD, TokenKind::Minus=>Op::SUB, TokenKind::Star=>Op::MUL, TokenKind::Slash=>Op::DIV,
                           TokenKind::EqEq=>Op::EQ, TokenKind::BangEq=>Op::NE, TokenKind::Lt=>Op::LT, TokenKind::Le=>Op::LE,
                           TokenKind::Gt=>Op::GT, TokenKind::Ge=>Op::GE, _=>unreachable!() });
    }
    Expr::Call { callee, args } => {
      self.emit_expr(callee);
      for a in args { self.emit_expr(a); }
      self.emit(Op::CALL); self.emit_u8(args.len() as u8);
    }
  }
}
```

### 9.2 Control flow (patching)

```rust
fn emit_if(&mut self, cond: &Expr, then_s: &Stmt, else_s: Option<&Stmt>) {
  self.emit_expr(cond);
  self.emit(Op::JUMP_IF_FALSE);
  let jf = self.emit_u16_placeholder(); // record position
  self.emit(Op::POP);
  self.emit_stmt(then_s);
  self.emit(Op::JUMP);
  let je = self.emit_u16_placeholder();
  self.patch_u16(jf, self.here_offset_from(jf));
  self.emit(Op::POP);
  if let Some(e) = else_s { self.emit_stmt(e); }
  self.patch_u16(je, self.here_offset_from(je));
}
```

---

## 10) CLI behavior (v0)

* `basilc run examples/expr.basil` → lex/parse/compile/execute.
* `basilc repl` → interactive (line → compile → run frame).
* `basilc dump --tokens file` | `--ast` | `--bc` → debugging.

---

## 11) Example programs

**examples/hello.basil**

```basil
PRINT "Hello, Basil!";
```

**examples/expr.basil**

```basil
LET a = 2 + 3 * 4;
PRINT a;         // 14
LET b = (2 + 3) * 4;
PRINT b;         // 20
```

**examples/fib.basil**

```basil
FUNC fib(n)
BEGIN
  IF n < 2 THEN RETURN n;
  RETURN fib(n - 1) + fib(n - 2);
END

PRINT fib(10); // 55
```

---

## 12) Testing strategy

* Unit tests per crate (lexer, parser, compiler, vm).
* Golden tests: source → bytecode hex dump → compare.
* E2E: run examples and verify stdout.

---

## 13) Roadmap from here

1. **Check in skeleton**: crates, opcodes, minimal lexer, numeric literals, string interner.
2. **Implement Pratt parser** and statements `LET/IF/WHILE/RETURN/BLOCK`.
3. **Bytecode compiler** with patching for jumps; functions + call frames.
4. **Builtins**: `PRINT`, `clock()`; error reporting with spans.
5. **Release v0.1** with examples + docs.
6. **Next**: Booleans short‑circuit, arrays/maps, for‑loops, file I/O, import/module system.
7. **Then**: async runtime scaffold; C‑ABI & WASI plugin MVP; Postgres driver.
8. **Finally**: register/SSA IR and C/WASM/JS emitters.

---

## 14) Licensing & contribution

* MIT or Apache‑2.0 for maximum adoption.
* `CONTRIBUTING.md` with rustfmt + clippy gates; CI on Windows/Linux.

---

**End of v0 plan.** When you’re ready, we can generate the initial crates, opcodes, and example files so you can `cargo run -p basilc -- examples/hello.basil` and see the first output.

🐖🐷🌿💻🎉🚀