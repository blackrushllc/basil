# Basil 🌿

## This is the Basil Programming Language
> ### This is what first year students should learn.
> ### This is what hobbyists should learn.
> ### This is what professionals should learn.
> ### This is the only programming language you need.

>
> Invite link to Blackrush Slack (Expires 11/13/25)
>
> https://join.slack.com/t/blackrushworkspace/shared_invite/zt-3g33s1rxc-9wWmCfggBEzInblqjzsn1A
>
> Join the Blackrush Slack Community for daily builds, discussions, lols
>


🌱 Basil is a Modern, Mod-able, AI-aware, Object Oriented (or not) BASIC language Bytecode Interpreter and **Cross-Platform
Compiler** with lots of Rad Mods such as AI, AWS, Zip, Crypt (Base64, PGP) CrossTerm, Inet (SMTP, FTP, Json, Curl, REST, etc), 
SQL(MySQL/Postgres, RDS, Sqlite, ORM, etc), MIDI (Audio, DAW), and even a Totally Tubular "OK Prompt" CLI mode 
(Jolt Cola not included)

>
> Complete Online Reference: https://yobasic.com/basil/reference.html
>
> Look at the /docs/ folder for guides, development notes, and more.
>

🌱 Basil is the first high level interpreted and compiled language designed with AI in mind, from code generation, module building, and even adding features or fixing bugs in itself!

🌱 Basil programs can be compiled to native binaries for Windows, Linux, and MacOS. Basil can also run as a CGI script templating engine
using \<?basil .. ?> tags like Php.  

🌱 Basil is easier learn and to use, and much faster than Python or Php.

🌱 Basil has all of the simplicity of old BASIC, but with all of the features of modern languages like Python or Go, plus the power of AI.

🌱 Basil is a fantastic "First Programming Language", but powerful enough to be used for anything.

🌱 Basil includes lots of sample programs including a complete Website Framework and MIDI DAW application.

🌱 Basil puts the power of AI in the hands of the student, hobbyist, and professional programmer.

🌱 Basil includes Built-in support ("Mods") for AI, Audio, MIDI, AWS (s3, ses, sqs, etc), CSV, CURL, JSON, SQL, Zip, Base64, PGP (encryption), REST, SFTP, SMTP and more

🌱 Basil includes the ability to have your favorite AI train itself on how to make Basil Library Objects ("Mods") and write Basil code.

🌱 Read the previous line again

🌱 Basil has a built-in ORM (Object Relational Mapping) for SQL databases.

🌱 Basil has a Rad 1980's GWBASIC interface if you want to use that. You can even instantiate Classes for manual testing, enter a program (with line numbers!) and run it, save it, load it, list it, and more. Cowabunga!

🌱 Basil has LOTS of documentation, examples, and a lot of old school and new school fun. 

🌱 Basil is inspired by Bob Zale's PowerBASIC and the warmth and simplicity of BASIC, 
but reimagined for today's developer needs with modern features, a robust standard library, 
and seamless interoperability with C and WebAssembly (WASI).

🌱 Basil is written in Rust for safety and performance, and aims to provide a delightful developer experience.

🌱 Basil is open source under the MIT or Apache-2.0 license.

🌱 Basil is a project by Blackrush LLC, Tarpon Springs, Florida, and written by Erik Olson.

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

🌿 Running a basil program with a full build (all libraries) (Recommended)

```terminal
cargo run -q -p basilc --features obj-all -- run examples/objects.basil
```

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

(BADLY needs to be updated)

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

>
> Terminal control (obj-term):
>
> - Enable the terminal feature and run examples:
    >  cargo run -q -p basilc --features obj-term -- run examples/term/01_colors_and_cls.basil
>
> - New commands when enabled:
    >  CLS, CLEAR, HOME, LOCATE(x%, y%), COLOR(fg, bg), COLOR_RESET, ATTR(bold%, underline%, reverse%), ATTR_RESET,
    >  CURSOR_SAVE, CURSOR_RESTORE, TERM_COLS%(), TERM_ROWS%(), CURSOR_HIDE, CURSOR_SHOW, TERM_ERR$()
>
> Color values for COLOR can be 0..15 or names (case-insensitive):
> 
> 0=Black, 1=Red, 2=Green, 3=Yellow, 4=Blue, 5=Magenta, 6=Cyan, 7=White, 8=Grey,
> 9=BrightRed, 10=BrightGreen, 11=BrightYellow, 12=BrightBlue, 13=BrightMagenta, 14=BrightCyan, 15=BrightWhite
>
> Names: "black","red","green","yellow","blue","magenta","cyan","white","grey",
"brightred","brightgreen","brightyellow","brightblue","brightmagenta","brightcyan","brightwhite"
>
> Examples are in examples/term/.
>

### Complete list of Basil Feature Objects (Mods). You can link them individually or all at once with --features obj-all (Recommended)

At build time, you can enable any of the following mods.  You can also enable all of them at once with --features obj-all.

Enabling more mods will increase the size of the Basil binary, but will give you more functionality.

Enabling mods will add new commands and functions to Basil.

Some mods are automatically bundled with other mods when there is interoperability, such as obj-orm requires obj-sql, etc 

+ obj-ai - Enable AI commands and functions in Basil
+ obj-audio - Audio playback and recording (Alone)
+ obj-aws - S3, SES, SQS, etc
+ obj-base64 - Base64 encoding and decoding
+ obj-bmx - An example set of Basil Modules for you to use as a starting point.
+ obj-crypto - PGP, other encryption and decryption tools
+ obj-csv - CSV
+ obj-curl - Curl client
+ obj-daw - All Midi and Audio related objects
+ obj-inet - Internet client (HTTP, FTP, SMTP, REST, etc)
+ obj-json - JSON utilities
+ obj-midi - MIDI audio playback and recording (Alone)
+ obj-mysql - MySQL
+ obj-net - SFTP, SMTP, REST, etc
+ obj-orm - Object Relational Model
+ obj-pgp - PGP encryption and decryption (alone)
+ obj-postgres - Postgres
+ obj-rds - RDS
+ obj-rest - REST API client
+ obj-sftp - SFTP client (alone)
+ obj-smtp - SMTP client (alone)
+ obj-sql - SQL (MySQL, Postgres, RDS, etc)
+ obj-sqlite - SQLite
+ obj-term - Terminal control using CrossTerm
+ obj-zip - Zip file compression and decompression
+ **obj-all - Enable all of the above (Recommended)**