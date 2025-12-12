# Basil 🌿

Download runnable binaries for Windows, Linux and MacOS here:

https://github.com/blackrushllc/basil/releases/tag/Beta

## This is the Basil Programming Language
> ### This is what first year students should learn.
> ### This is what hobbyists should learn.
> ### This is what professionals should learn.
> ### This is the only programming language you need.
> ### Basic-Like or C-Like syntax, but modern.
> ### AI-Aware from the ground up.
> ### Develop for the web with Basil-Serve and Php-like tags.

>
> Invite link to Blackrush Slack (Never Expires)
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
> Complete Online Reference: https://basilbasic.com/basil/reference.html
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

🌱 Basil has a built-in ORM (Object Relational Mapping) for SQL databases.

🌱 Basil has Php-like CGI templating with Laravel Blade-Like view rendering.

🌱 Basil has a Rad 1980's GWBASIC interface if you want to use that. You can even instantiate Classes for manual testing, enter a program (with line numbers!) and run it, save it, load it, list it, and more. Cowabunga!

🌱 Basil has LOTS of documentation, examples, and a lot of old school and new school fun. 

🌱 Basil is inspired by Bob Zale's PowerBASIC and the warmth and simplicity of BASIC, 
but reimagined for today's developer needs with modern features, a robust standard library, 
and seamless interoperability with C and WebAssembly (WASI).

🌱 Basil is written in Rust for safety and performance, and aims to provide a delightful developer experience.

🌱 Basil is open source under the MIT or Apache-2.0 license.

🌱 Basil is a project by Blackrush LLC, Tarpon Springs, Florida, and written by Erik Olson.

🌱 Basil is pronounced like "basil" the herb, and is a pun volcano.

## Why first languages matter

Your first programming language shouldn’t be a puzzle box. It should:
- Lower cognitive load while you’re learning core ideas like variables, expressions, control flow, and functions.
- Offer clear, immediate feedback (short edit–run cycles, gentle error messages).
- Be consistent in how it uses syntax to express ideas.
- Build habits that transfer to the broader programming world.

Basil🌿 was designed against these criteria. It keeps the classic readability of BASIC, but adds an alternate “modern” surface syntax so that what you learn today still looks familiar later.

At the same time, Basil🌿 is powerful enough to build real projects, with a growing standard library and a modular “mod” system that to adds out-of-box functionality like AI, AWS, SQL databases, HTTP, SMTP, JSON, CSV, cryptography, audio/MIDI/DAW support, and more.

Basil🌿 is also made for the AI age, the first programming language designed for AI from the ground up.

---

### Two ways to say the same thing (both valid in Basil🌿)
Classic BASIC style:

```
REM BOTH SYNTAXES ARE VALID:

REM Infinite loop with BREAK (will break at 3)
LET i = 0;
WHILE TRUE BEGIN
    LET i = i + 1;
    IF i == 3 THEN BEGIN // Block IF
        BREAK;
    END
    PRINT i;
END
```

Modern brace style (THEN is implied when you open a brace):

```
// Infinite loop with BREAK (will break at 3)
let i = 0;
while true {
    let i = i + 1;
    if i == 3 { // Block IF
        break;
    }
    print i;
}
```

You can mix and match styles in one program. Internally, both forms compile to the same structures and run the same way.

---


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

🌿 https://basilbasic.com - The website for Basil

🌿 https://basilbasic.com/basil/basil.html - The original 15 Minute Presentation Handout (nicer one below)

🌿 https://basilbasic.com/basil/cgi.basil - Live BASIL CGI demo (just to prove it works)

🌿 https://basilbasic.com/basil/reference.html - comprehensive Basil Language Reference (kept current)

🌿 https://basilbasic.com/basil/hello.basil - literally just a PRINT "Hello" with no CGI anything (just to prove it works) 

🌿 https://basilbasic.com/basil/website/index.basil - A simple Basil CGI web app with login, register, user home, logout

🌿 https://YoBASIC.com/basic - A free online BASIC tutorial site by Blackrush LLC

# The Basil Programming Language for Education

### Why Basil🌿 works as a first learning language
- Gentle, explicit control flow
    - `if ... then` and `if ... { ... }` are both accepted; `else/elseif` read naturally.
    - `while`, `for`, and `select case` are straightforward and visible.
- Clear block boundaries
    - You can choose `BEGIN ... END` or `{ ... }`. Either way, blocks are explicit and obvious.
- Low ceremony, fast feedback
    - Small surface area, immediate execution, simple I/O (`print`, `println`).
- Case‑insensitive keywords; readable by design
    - Beginners don’t lose momentum over capitalization or minor formatting.
- A bridge to mainstream languages
    - The brace form prepares students to read/write C‑family languages without abandoning BASIC’s clarity.

---


### How Basil🌿 addresses first‑year pain points
- Visible structure
    - Choose braces or `BEGIN/END`. Students can literally “see the block.”
- Predictable, explicit control flow
    - `if/elseif/else`, `while`, `for/next`, and `select case` have minimal hidden rules.
- One concept at a time
    - You can start with the classic style and later migrate to braces without relearning the language.
- Transferable skills
    - The modern style maps cleanly to C, C#, Java, JavaScript, and Go idioms.
- Friendly diagnostics
    - Errors mention both classic and modern forms (e.g., “Expected THEN or ‘{’ after IF condition.”), guiding students instead of stopping them.

---

### A suggested path for an intro course (e.g., COP‑1000)
1. 🌱 Week 1–2: Variables, arithmetic, `print`/`println`, simple `if/then`.
2. 🌱 Week 3: Loops (`while`, `for/next`), `break` and `continue`.
3. 🌱 Week 4: Functions (`func`, `return`), parameters, local scope.
4. 🌱 Week 5: Decisions at scale: `select case`; string operations.
5. 🌱 Week 6: Modernization—introduce the brace style in parallel; show side‑by‑side translations.
6. 🌱 Week 7+: Objects and modules as applicable; project work.

Students leave with working mental models and syntax that looks familiar across the industry.

---

### Quick syntax map: classic to modern
- IF
    - Classic: `IF cond THEN BEGIN ... END`
    - Modern:  `if cond { ... }`
- ELSE / ELSEIF
    - Classic: `ELSE BEGIN ... END` or single statement
    - Modern:  `} else if cond { ... } else { ... }`
- WHILE
    - Classic: `WHILE cond BEGIN ... END`
    - Modern:  `while cond { ... }`
- FOR / NEXT
    - Classic: `FOR i = 1 TO 10 ... NEXT i`
    - Modern:  same control header; body can use `{ ... }`
- SELECT CASE
    - Classic: `SELECT CASE x ... END [SELECT]`
    - Modern:  `select case x { ... }`

Both forms are always valid; pick one or mix as you learn.

---

### Basil🌿 is a cross-platform Interpreter and Native-Code Compiler

- Basil🌿 is a cross-platform interpreter and native-code compiler. It runs on Windows, Linux, and macOS.
- Basil🌿 can run source files, or compile to a single portable executable that runs without installing a runtime or dependencies.

### A Large Standard Extended Library with "Mods"

Basil🌿  contains a robust standard library and pre-built "Basil🌿 Feature Objects" or "Mods" that provide real-world
functionality out of the box. These include:

- 🌿 File I/O
- 🌿 String manipulation
- 🌿 Date and time functions
- 🌿 Math functions
- 🌿 Artificial Intelligence (AI) integration
- 🌿 Networking (HTTP, SMTP, CURL)
- 🌿 Database access (SQLite, SQL, ORM Wrappers)
- 🌿 JSON and CSV handling
- 🌿 Web Development (Templating HTML with embedded <?Basil🌿 .. ?> like Php)
- 🌿 Laravel Blade-like view rendering
- 🌿 AWS integration
- 🌿 Advance Screen UI (CrossTerm)
- 🌿 AI/ML interfaces
- 🌿 Audio/MIDI/DAW support
- 🌿 Cryptography (Base64, PGP, Zip)
- 🌿 WebAssembly support
- 🌿 Example starter Mods
- 🌿 Tons of examples and documentation
- 🌿 AI Onboarding for Rust Developers
- 🌿 Community support
- 🌿 and more!


### Looking ahead

Basil🌿  keeps backward compatibility while also adding new features.  Recently added features include:

- 🌱 List and dictionary literals.
- 🌱 User‑defined types (`TYPE ... END TYPE`).
- 🌱 Fixed‑length strings where appropriate.
- 🌱 Game-capable graphics
- 🌱 Asterisk Integration (VoIP)
- 🌱 WebAssembly (WASM) support
- 🌱 Distributed processing (DPROC)
- 🌱 Interop with Rust, Go, and C#
- 🌱 Laravel Blade-like view rendering

At the time of this writing we are also working on:
- 🌱 An web-based tutorial IDE (Integrated Development Environment)
- 🌱 JetBrains integration
- 🌱 VS Code integration
- 🌱 Runtime loadable modules
- 🌱 Apache/Nginx integration

... and we are open to suggestions!

These features will slot into the existing language without disrupting the core learning experience.

### Education and Community

Basil🌿  is an open source project and is actively developed by a community of volunteers, built with education and community in mind.

We have built Basil🌿 to be a great learning tool for beginners, while remaining robust and powerful for real-world use.
We are committed to making it easy for you to learn the Basil🌿 language and to contribute to the project.

---

### Summary

Basil🌿 restores the simplicity many of us loved in our first encounters with BASIC, while offering a modern, brace‑style
path that aligns with today’s mainstream languages. It’s small enough to learn quickly, expressive enough to build real
projects, and friendly enough to keep students in the game—so more learners finish the course confident, not frustrated.

### Resources

Github Repository: https://github.com/blackrushllc/basil

Complete Online Reference: https://basilbasic.com/basil/reference.html

WIP, Interactive Online Wiki and reference (Written in Basil!): https://blackrushbasic.com/

Email: BlackrushDrive@Gmail.com

Everywhere: @BlackrushWorld

Basil is an open source project under MIT license, Copyright (c) 2026 Blackrush LLC, Tarpon Springs, Florida, USA.