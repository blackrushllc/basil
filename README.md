# Basil 🌿

🌱 A modern BASIC‑flavored language focused on web/back‑end with the 
ability to compile to bytecode or binaries, across all platforms, and includes lots of library modules
such as Zip, Base64, JSON, SQL, MIDI, AI, and more.

🌱 Basil is the first high level interpreted and compiled language designed with AI in mind, from code generation, module building, and even adding features or fixing bugs in itself!

🌱 Basil programs can be compiled
to native binaries for Windows, Linux, and MacOS. Basil can also run as a CGI script templating engine
using \<?basil .. ?> tags like Php.  

🌱 Basil is easier to use, and much faster than Python or Php.

🌱 Basil includes lots of sample programs including a complete Website Framework and MIDI DAW application.

🌱 Basil puts the power of AI in the hands of the student, hobbyist, and professional programmer.

🌱 Basil includes the ability to have your favorite AI train itself on how to make Basil Library mods and write Basil code.

🌱 Basil is inspired by Bob Zale's PowerBASIC and the warmth and simplicity of BASIC, 
but reimagined for today's developer needs with modern features, a robust standard library, 
and seamless interoperability with C and WebAssembly (WASI).

🌱 Basil is written in Rust for safety and performance, and aims to provide a delightful developer experience.

🌱 Basil is open source under the MIT or Apache-2.0 license.

🌱 Basil is a project by Blackrush LLC (https://blackrush.us).

🌱 Basil is pronounced like "basil" the herb, and is a pun volcano.

### Quick Try:

🌿 Running a basil program without rebuilding the VM:

```terminal
target/release/basilc run examples/hello.basil
# or
target/debug/basilc run examples/hello.basil
```

Building and deploying Basil to run CGI scripts on Linux:

```
cargo build --release -p basilc
install -m 0755 target/release/basilc /usr/lib/cgi-bin/basil.cgi
```

🌿 Running a basil program with a bunch of libraries

```terminal
cargo run -q -p basilc --features "obj-curl obj-zip obj-base64" -- run examples/zip_demo.basil
```

🌿 Running a basil program with a full build (all libraries)

```terminal
cargo run -q -p basilc --features obj-all -- run examples/objects.basil
```

Terminal control (obj-term):

- Enable the terminal feature and run examples:
  cargo run -q -p basilc --features obj-term -- run examples/term/01_colors_and_cls.basil

- New commands when enabled:
  CLS, CLEAR, HOME, LOCATE(x%, y%), COLOR(fg, bg), COLOR_RESET, ATTR(bold%, underline%, reverse%), ATTR_RESET,
  CURSOR_SAVE, CURSOR_RESTORE, TERM_COLS%(), TERM_ROWS%(), CURSOR_HIDE, CURSOR_SHOW, TERM_ERR$()

Color values for COLOR can be 0..15 or names (case-insensitive):
  0=Black, 1=Red, 2=Green, 3=Yellow, 4=Blue, 5=Magenta, 6=Cyan, 7=White, 8=Grey,
  9=BrightRed, 10=BrightGreen, 11=BrightYellow, 12=BrightBlue, 13=BrightMagenta, 14=BrightCyan, 15=BrightWhite
  Names: "black","red","green","yellow","blue","magenta","cyan","white","grey",
         "brightred","brightgreen","brightyellow","brightblue","brightmagenta","brightcyan","brightwhite"

Examples are in examples/term/.

See:
 + docs - all the docs, guides, development, etc
 + examples - lots of Basil program examples
 + examples/hello.basil - "Hello World" program
 + examples/website/ - a simple Basil CGI web app with login, register, user home, logout
 + Useful links:

🌿 https://yobasic.com - The website for Basil

🌿 https://yobasic.com/basil//basil.html - The original 15 Minute Presentation Handout (nicer one below)

🌿 https://yobasic.com/basil/cgi.basil - Live BASIL CGI demo (just to prove it works)

🌿 https://yobasic.com/basil/reference.html - comprehensive Basil Language Reference (kept current)

🌿 https://yobasic.com/basil/hello.basil - literally just a PRINT "Hello" with no CGI anything (just to prove it works) 

🌿 https://yobasic.com/basil/website/index.basil - A simple Basil CGI web app with login, register, user home, logout



# 🌱 Basil Prototype v0 — Public Plan & Skeleton

A minimal, public‑ready blueprint for a modern BASIC‑flavored language focused on web/back‑end. This plan targets a tiny, end‑to‑end slice: **source → tokens → Abstract Syntax Tree → bytecode → VM** with room to evolve into C/WASM/JS backends.

---

## 0) High‑level goals

* 🌱 **Developer joy**: BASIC warmth + modern features (expressions, async later, modules).
* 🌱 **Simple core now, room to grow**: start with a stack VM, evolve to register/SSA.
* 🌱 **Interop first**: design a stable C Application Binary Interface (ABI) and WASI component boundary (later phases).
* 🌱 **Linux + Windows, single binary toolchain**.

---

## 1) 🌱 Repository layout (Rust host)

(needs to be updated)

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
│  ├─ parser/        (Pratt parser → Abstract Syntax Tree)
│  ├─ ast/           (Abstract Syntax Tree nodes + spans)
│  ├─ compiler/      (Abstract Syntax Tree → bytecode chunk)
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

