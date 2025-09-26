# TADA!: Implement functions, calls, returns, control flow, and comparisons
# Goal: run this program:
```FUNC fib(n)
  IF n < 2 THEN
    RETURN n
  ENDIF
  RETURN fib(n - 1) + fib(n - 2)
END
PRINT fib(10)
``` 
### 🌿💻🎉🚀 This is working now 🐷🐷🐷🐷🐷🐷🌱 

1. **Functions** (`FUNC name(params) … END`)
2. **Calls** (`fib(10)`)
3. **Returns** (`RETURN n`)
4. **Control flow & comparisons** (`IF n < 2 THEN …`, and `<`)

Below is the smallest, realistic slice to make your `fib` run, and how to wire it.

---

# What to add (MVP to run `fib`)

## A) AST

Add nodes so we can represent functions, calls, returns, ifs, and blocks:

* **Expressions**

    * `Call { callee: Box<Expr>, args: Vec<Expr> }`
    * Extend `BinOp` to include `Eq, Ne, Lt, Le, Gt, Ge`

* **Statements**

    * `Func { name: String, params: Vec<String>, body: Vec<Stmt> }`
    * `Return(Option<Expr>)`
    * `If { cond: Expr, then_branch: Box<Stmt>, else_branch: Option<Box<Stmt>> }`
    * `Block(Vec<Stmt>)`

*(Your `PRINT` and `LET` stay as-is.)*

## B) Parser

* **Function declaration**
  `FUNC <ident> '(' [id {',' id}] ')' <block>`
  Block is `BEGIN { stmt } END`.

* **Calls (postfix)**
  After parsing a primary, loop on `(` to parse `Call`.

* **Comparisons**
  Extend Pratt table to recognize `== != < <= > >=` between additive and term levels.

* **Return**
  `RETURN [expr] ';'`

* **If**
  `IF expr THEN stmt [ELSE stmt]`
  (Your example does `RETURN` as the `stmt` on the same line; that works if `stmt` parses another statement.)

## C) Bytecode & Compiler

Add opcodes (tiny, stack-based):

```
# existing
ConstU8, LoadGlobal, StoreGlobal, Add, Sub, Mul, Div, Neg, Print, Pop, Halt

# new
Eq, Ne, Lt, Le, Gt, Ge
Jump(u16), JumpIfFalse(u16)
LoadLocal(u8), StoreLocal(u8)
Call(u8), Ret
```

**Functions**: compile a `FUNC` into a separate `Chunk` with `arity` and emit:

```
Const <FunctionObj> ; StoreGlobal <"fib">
```

**Calls**: compile `callee` then args → `Call argc`.

**Locals/params**: inside a function, resolve identifiers to **local slots** (params are slots `0..arity-1`). Keep a simple `HashMap<String, u8>` for locals per function. Fall back to globals if not found.

**If**: standard patching—`cond; JumpIfFalse -> else; <then>; Jump -> end; <else>;`.

**Return**: `expr; Ret` (or `Const null; Ret` if empty).

## D) VM (frames + locals)

Introduce **call frames**:

```rust
struct Frame {
  chunk: Chunk,   // or Rc<Chunk> if you want to avoid copies
  ip: usize,
  base: usize,    // start of this frame’s locals/args on the stack
}
```

VM holds `frames: Vec<Frame>`, `stack: Vec<Value>`, and `globals: Vec<Value>`.

* **CALL argc**:

    * Stack layout after compile: `[..., callee, arg1, ..., argN]`
    * `callee_idx = stack.len() - 1 - argc`
    * Take `callee = stack.remove(callee_idx)` (so args stay contiguous)
    * `base = stack.len() - argc`
    * Push a new `Frame { chunk: callee.chunk.clone(), ip: 0, base }`

* **RET**:

    * pop return value
    * pop the current frame
    * shrink stack to `frame.base`
    * push the return value

* **LOAD_LOCAL idx**: push `stack[frame.base + idx]`

* **STORE_LOCAL idx**: `stack[frame.base + idx] = pop()`

* **JUMP / JUMP_IF_FALSE**: adjust `ip` by a signed/unsigned offset

* **Comparisons**: numeric for now is enough for `n < 2`

---

# Why this is the right cut

It’s the smallest set that:

* supports **recursion** (stack frames),
* avoids a GC change (values are POD + owned `Chunk` for now),
* compiles `fib` without touching modules/imports,
* and keeps the VM simple and debuggable.

---

# Order of operations (practical)

1. **AST + Parser**: add `Func`, `Call`, `Return`, `If`, `Block`, comparisons, and call postfix loop.
2. **Bytecode + Compiler**: add new opcodes; function compiler that creates a `Function` value; locals map for params; `if` jump patching; `return`.
3. **VM**: convert to frame-based loop; implement `CALL`/`RET`, `LOAD/STORE_LOCAL`, `JUMP/JIF`, comparisons.
