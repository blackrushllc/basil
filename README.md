# Basil 🌿
A modern BASIC‑flavored language focused on web/back‑end with the 
ability to compile binaries, transpile to C, NodeJS and WebAssembly, 
and leverages a lot of existing libraries from other environments.

🌱 Basil is inspired by Bob Zale's PowerBASIC and the warmth and simplicity of BASIC, 
but reimagined for today's developer needs with modern features, a robust standard library, 
and seamless interoperability with C and WebAssembly (WASI).

🌱 Basil is written in Rust for safety and performance, and aims to provide a delightful developer experience.

🌱 Basil is open source under the MIT or Apache-2.0 license.

🌱 Basil is a project by Blackrush LLC (https://blackrush.us).

🌱 Run a Basil script as CGI like: https://yobasic.com/basil/hello.basil

### Quick Try:

🌿 Running a basil program without rebuilding the VM:
```terminal
target/release/basilc run examples/hello.basil
# or
target/debug/basilc run examples/hello.basil
```

🌿 Running a basil program with the BMX example objects enabled:
```terminal
cargo run -q -p basilc --features obj-bmx -- run examples/hello.basil
# or
cargo run -q -p basilc --features "obj-bmx-rider obj-bmx-ream" -- run examples/hello.basil
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

Interesting files to read in this repo:

+ docs/BASIL_KEYWORDS_BY_CATEGORY.md
+ docs/BASIL_REFERENCE.md
+ docs/FILE_ID.md
+ AI_FEATURE_REQUEST.md - Paste this whole thing into ChatGPT FIRST, so it knows what Basil Objects are, and then you can ask it generate a Junie Ultimate prompt for a new feature object !!!
+ ASTERISK.md - Using Basil with Asterisk
+ BASIL.md - 15‑Minute Presentation Handout
+ BASIL_CGI.md - CGI Support for Basil (the original proposal) (this has been implemented)
+ BEGIN_END.md - All about using BEGIN..END and why and stuff
+ CLASSES.md - CLASS Support for Basil (the original proposal) (this has been implemented)
+ CLASSES_JUNIE.md - Prompt for Junie — Implement CLASS Feature in Basil (She does the hard work!)
+ CONTRIBUTING.md - How to Contribute
+ ELIZA.md - How the original 1966 ELIZA works (science yeah!)
+ FILE_IO.md - FILE IO Support for Basil (the original proposal) (this has been implemented)
+ FUNCTIONS.md - TADA!: Implement functions, calls, returns, control flow, and comparisons
+ GOALS.md - My Day 1 proposal goals for Basil
+ IDE.md - How to configure Geany or VS-Code for Basil (and others soon)
+ JETBRAINS.md - Work in progress on a Jetbrains IDE Plugin for Basil (And Yore) ([What is Yore?](https://github.com/blackrushllc/yore))
+ LIBRARY_OBJECTS.md - A reference for the add-on function libraries
+ OBJECTS.md - Object Library Support for Basil (the original proposal) (this has been implemented)
+ PRESENTATION.md - 15‑Minute Stand‑Up Demo Notes (for Teachers) also here: [presentation.pdf](https://github.com/blackrushllc/basil/blob/main/presentation.pdf)
+ PUNS.md - ChatGPT said "Basil" is a "Pun Volcano" and this doc proves it
+ README.md - This file here
+ STUDENTS.md - How the Basil BASIC Interpreter Works (for students) also here: [basil_in_a_nutshell.pdf](https://github.com/blackrushllc/basil/blob/main/basil_in_a_nutshell.pdf)
+ TECHNICAL.md - Basil Technical Architecture and Execution Model (for rocket scientists)
+ TODO.md - A laughably old Todo list
+ VISION.md - Part of my original proposal for Basil
+ WASM.md - Building Basil for WebAssembly (WASM)
+ WASM2.md - Running BASIL in the browser with WASM
+ WEB_PAGES.md - Basil CGI / Web Templating Design (the original proposal) (this has been implemented)

See the examples folder for lots of cool example Basil scripts. These were mostly produced to test each feature as it was added and then rest everything else so we know we didn't break anything

### Known limitations

- Interactive console input (INPUT\$, INPUTC\$, INKEY\$, INKEY%): On Windows, these functions require a real
  console/TTY. When running basilc via an IDE’s Run/Debug console (e.g., RustRover Run menu), the IDE’s pseudo-terminal
  may not support raw-mode keyboard polling and can cause hangs or repeated key echo. Run interactive examples from a
  regular terminal (PowerShell, cmd.exe ("DOS" lol), Putty, terminal window, etc) for correct behavior. Non-interactive
  scripts and normal terminal usage are unaffected.
- If you must run from an IDE, configure the run target to use an external console or disable input features in your
  script.
- lythere is a "test" option to run non-interactively, mock input, and output comments
- There is no maximum number of lines of code in a script.
- There is no maximum number of variables, functions, objects, or arrays.
- There are no limits to the size of strings variables, arrays, or objects.
- ^ in theory

# TODO next

🌿 Add packages like ~~Zip~~, ~~Base64~~, JSON, XML, HTTP, SQLite, MySQL, PostgreSQL, Redis, LDAP, SMTP, IMAP, FTP, SFTP, SSH, TLS/SSL, WebSockets, JWT, OAuth2, AWS SDK, Azure SDK, GCP SDK

🌿 Need to Add STR, VAL, SGN, INT, SQR, RND, RNDM, SIN, COS, TAN, ATN, EXP, LOG, SINH, COSH, TANH, ASIN, ACOS, ATAN, SQRT, RINT, FIX, EXPONENTIAL, REPLACE, STRTOK, OCT, HEX, INSTR, FIND, STR$

# 🌱 WHAT'S NEW 🌱

### 🌿 STATUS UPDATE _AI support_ !!!

* AI support added
* See examples/ai for several examples
* See AI.md and other AI docs in here for more info and guides
* This is different from the "AI ONBOARDING" support which lets you train AI on how to make Basil mods and write Basil code
* This is actual AI stuff you can put in your BASIC programs

### 🌿 STATUS UPDATE _Midi support_ !!!

* Midi support added
* See examples/midi.basil for an example
* See MIDI.md for more info

### 🌱 Coming soon: A combination of AI and MIDI which is going to be lit

### 🌿 STATUS UPDATE _CLASSES_ !!!

+ Classes are now supported
+ See examples/classes.basil to see how to instantiate and use classes
+ See examples/my_class.basil to see how to define a class
+ Classes can have public variables and public functions
+ Classes can be instantiated and interacted with from BASIL scripts
+ Classes are defined in separate files, and can be instantiated from BASIL scripts with the DIM statement.
+ Class files are compiled into bytecode and stored in a .basilx file.
+ Class files can be distributed without needing to share the source code.
+ See CLASSES.md and CLASSES_JUNIE.md for more info

Example usage:

````basil
REM Demo: Using CLASS to instantiate and interact with a class instance

DIM user@ AS CLASS("my_class.basil");

REM Access and modify a public variable
PRINTLN "Initial description:", user@.Description$;
LET user@.Description$ = "These are my favorite users.";
PRINTLN "Updated description:", user@.Description$;

REM Call public functions
user@.AddUser("Erik");
user@.AddUser("Junie");
user@.AddUser("ChatGPT");

PRINTLN "User count:", user@.CountMyUsers%();

````

````basil
REM Example Basil Class: my_class.basil
REM Demonstrates public variables and public functions

DIM UserNames$(100);
LET Description$ = "Default description";

FUNC AddUser(name$)
BEGIN
  REM Adds the given name to the first empty slot
  FOR i% = 0 TO 99
    IF UserNames$(i%) == "" THEN
      BEGIN
        LET UserNames$(i%) = name$;
        RETURN
      END
  NEXT
END

FUNC CountMyUsers%()
BEGIN
  REM Returns the number of non-empty user names
  LET count% = 0
  FOR i% = 0 TO 99
    IF UserNames$(i%) != "" THEN
      BEGIN
        LET count% = count% + 1;
      END
  NEXT
  RETURN count%;
END

````

### 🌿 STATUS UPDATE _"test" mode_ !!!

+ "test" CLI command to run non-interactively, mock input, and output comments

Example usage:

```
 cargo run -q -p basilc --features obj-bmx -- test examples/input.basil

```

Output:

```

> cargo run -q -p basilc --features obj-bmx -- test examples/input.basil

COMMENT: Demo of INPUT$ and IF statements.
COMMENT: LET A$ = INPUT$("\nEnter your name:");

Hello, Bob!
Do you want to do something else? (Y/N): YMock input to INPUTC$ given as Y
COMMENT: Block IF:

Winken
BLinken
Nod

COMMENT: Immediate IF:
You said yes
Goodbye!

```

### 🌿 STATUS UPDATE _Bytecode is compiled and stored in a .basilx file_ !!!
+ Bytecode automatically recompiles whenever the source file is changed
+ Bytecode is stored in a .basilx file
+ Bytecode runs faster than the original source code
+ Bytecode is portable between platforms (Windows, Linux, MacOS) using the Basil VM
+ You can distribute the .basilx file without needing to share the original source code



### 🌿 STATUS UPDATE _CGI scripts run like Php scripts with \<?basil .. ?>_ !!!

```html
#CGI_NO_HEADER
<?basil
  // Manual header mode: send headers explicitly, then a blank line
  PRINT "Status: 200 OK\r\n";
  PRINT "Content-Type: text/html; charset=utf-8\r\n\r\n";
?>
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>Basil CGI Demo</title>
</head>
<body>
  <h1>Hello, World</h1>
  <p>This page is rendered by a Basil CGI template.</p>

  <h2>Request parameters</h2>
  <p>Any GET or POST parameters will be listed below.</p>
  <ul>
  <?basil
    FOR EACH p$ IN REQUEST$()
      PRINT "<li>" + HTML$(p$) + "</li>\n";
    NEXT
  ?>
  </ul>
</body>
</html>
```

### 🌿 STATUS UPDATE _Lots of bug fixes and improvements, better error msgs_ !!!

### 🌿 STATUS UPDATE _Added INPUTC$, PRINTLN_, updated examples !!!

### 🌿 STATUS UPDATE _Added UCASE, LCASE, TRIM, STR, ASC, CHR, LEFT, RIGHT, MID, INSTR, FIND, STR$ and INKEY%_ !!!

### 🌿 STATUS UPDATE _Added WHILE Loop, BREAK, CONTINUE and Boolean Constants_ !!!

````basil

LET x = 0;
WHILE x < 3 BEGIN
    PRINT x;
    LET x = x + 1;
END

' Infinite loop with BREAK (will break at 3)
LET i = 0;
WHILE TRUE BEGIN
    LET i = i + 1;
    IF i == 3 THEN BEGIN // Block IF
        BREAK;
    END
    PRINT i;
END

' Using CONTINUE (skip 3)
LET j = 0;
WHILE j < 5 BEGIN
    LET j = j + 1;
    IF j == 3 THEN BEGIN
        CONTINUE;
    END
    PRINT j;
END

' Infinite loop with BREAK (will break at 3)
LET i = 0;
WHILE TRUE BEGIN
    LET i = i + 1;
    IF i == 3 THEN BREAK; // Immediate IF
    PRINT i;
END

' Using CONTINUE (skip 3)
LET j = 0;
WHILE j < 5 BEGIN
    LET j = j + 1;
    IF j == 3 THEN  CONTINUE;
    PRINT j;
END


' FALSE as never-enter condition
WHILE FALSE BEGIN
    PRINT "You should never see this";
END


````

### 🌿 STATUS UPDATE _Arrays of Objects and FOR/EACH Loops_ !!!

````basil

#USE BMX_RIDER, BMX_TEAM

DIM riders@(2) AS BMX_RIDER;
LET riders@(0) = NEW BMX_RIDER("Alice", 17, "Expert", 12, 3);
LET riders@(1) = NEW BMX_RIDER("Bob",   21, "Expert",  8, 9);
LET riders@(2) = NEW BMX_RIDER("Carol", 19, "Pro",    30, 4);

FOR EACH r@ IN riders@
  PRINT "Rider - ", r@.Describe$();
NEXT

DIM nums%(4);
FOR EACH n% IN nums%
  LET nums%(n%) = n% * n%;
NEXT

DIM t@ AS BMX_TEAM("Rocket Foxes", 2015, PRO);
t@.AddRider(riders@(0)); t@.AddRider(riders@(1)); t@.AddRider(riders@(2));

FOR EACH name$ IN t@.RiderNames$()
  PRINT name$;
NEXT

FOR EACH desc$ IN t@.RiderDescriptions$()
  PRINT desc$;
NEXT

````

### 🌿 STATUS UPDATE _Objects Working_ !!!
+ (Windows) ```cargo run -q -p basilc --features obj-bmx -- run examples\objects.basil```
+ (Linux) ```cargo run -q -p basilc --features obj-bmx -- run examples/objects.basil```
+ See OBJECTS.md for more info
+ Refer to the link below for my feelings on this new feature:
+ https://youtube.com/clip/UgkxBpXcWlbjLM0n_YrEbR__yWX6a-gF8yOl?si=dvQHvcfwzRZ-yIC4

#### Link in Rust objects crate:
```cargo run -q -p basilc --features obj-bmx -- run examples/objects.basil```

#### Link in C objects crate:
```cargo run -q -p basilc --features obj-c -- run examples/objects.basil```

Reference objects in BASIL:

```
REM Objects demo: BMX_RIDER and BMX_TEAM
REM This example assumes object support is compiled in with features enabling BMX_RIDER and BMX_TEAM.

#USE BMX_RIDER, BMX_TEAM

DIM riders@(2) AS BMX_RIDER;
LET riders@(0) = NEW BMX_RIDER("Alice", 17, "Expert", 12, 3);
LET riders@(1) = NEW BMX_RIDER("Bob",   21, "Expert",  8, 9);
LET riders@(2) = NEW BMX_RIDER("Carol", 19, "Pro",    30, 4);

FOR EACH r@ IN riders@
  PRINT "Rider - ",r@.Describe$();
NEXT

DIM nums%(4);
FOR EACH n% IN nums%
  LET nums%(n%) = n% * n%;
NEXT

DIM t@ AS BMX_TEAM("Rocket Foxes", 2015, PRO);
t@.AddRider(riders@(0)); t@.AddRider(riders@(1)); t@.AddRider(riders@(2));

FOR EACH name$ IN t@.RiderNames$()
  PRINTLN name$;
NEXT

FOR EACH desc$ IN t@.RiderDescriptions$()
  PRINTLN desc$;
NEXT

```



### 🌿 STATUS UPDATE _Arrays Working_ !!!
+ String, Integer and Float arrays up to 4 dimensions
+ Array function LEN() returns number of elements in array
+ Array command DIM creates/recreates array with specified dimensions

+ Also added INPUT, INKEY%, INKEY\$ functions for keyboard input
+ Syntax error output shows line number

TODO in this vein:
+ Array functions MID, LBOUND, UBOUND, REDIM, REDIM PRESERVE, REDIM SHARED
+ Copy / Slice etc
+ Maybe Array functions SORT, RESIZE, RESHAPE, TRANSPOSE, RANDOMIZE, REVERSE, FOR EACH Loops
+ Maybe Array types Byte(), Word(), Long(), Double()

```

REM Arrays example for BASIL
REM Demonstrates DIM for string ($), integer (%), and float arrays, with up to 2 dimensions

REM --- 1D integer array (0..5 inclusive => length 6) ---
DIM N%(5);
LET N%(0) = 10;
LET N%(5) = 99;
PRINT "N%(0)=", N%(0), ", N%(5)=", N%(5);
PRINT "LEN(N%)=", LEN(N%);

REM --- 1D float array (0..3 inclusive => length 4) ---
DIM X(3);
LET X(0) = 1.5;
LET X(3) = 2.5;
PRINT "X(0)=", X(0), ", X(3)=", X(3);
PRINT "LEN(X)=", LEN(X);

REM --- 2D string array (0..2 by 0..1 => 3 x 2 = 6 elements) ---
DIM S$(2,1);
LET S$(0,0) = "Hello";
LET S$(2,1) = "World";
PRINT "S$(0,0)=", S$(0,0), ", S$(2,1)=", S$(2,1);
PRINT "LEN(S$)=", LEN(S$);

REM Show that re-DIM resets the array
DIM S$(1,0); REM now capacity is 2 x 1 = 2 elements, previous contents cleared
LET S$(1,0) = "Reset";
PRINT "After re-DIM, LEN(S$)=", LEN(S$), "; S$(1,0)=", S$(1,0);


```

### 🌿 STATUS UPDATE _FOR/NEXT Loops Working_ !!!
+ Also String and Integer variable types (A$, MyNum%)
+ String concatenation with "+" (direct) or ", " (tab)
+ String functions LEN, LEFT\$, RIGHT\$, MID\$, INSTR
+ Example programs added

TODO in this vein:
+ String functions UPPER, LOWER, LTRIM, RTRIM, TRIM
+ String functions ASC, CHR, OCT, HEX
+ Functions VAL, SGN, INT, SQR, RND, RNDM, SIN, COS, TAN, ATN, EXP, LOG, SINH, COSH, TANH, ASIN, ACOS, ATAN, SQRT, RINT, FIX, EXPONENTIAL
+ String functions REPLACE, STR, STRTOK

```

FOR i = 1 TO 5
    PRINTLN i;
NEXT i;

FOR j = 5 TO 1 STEP -1
    BEGIN
        PRINT j;
        FOR i = 1 TO 5
            PRINTLN i;
        NEXT i;
    END
NEXT j;



```


### 🌿 STATUS UPDATE _CGI Working_ !!!

+ See bigger example above and in examples/cgi.basil etc
+ Apache config is described further below
+ Runs Along side html and Php files like normal

🌱 Basil executable can tell the difference between running in CLI mode vs web mode, and you can 
write Basil scripts that respond to HTTP requests or run as normal CLI programs.

# Build Basil From the command line on Linux:

```
cargo build --release -p basilc

install -m 0755 target/release/basilc /usr/lib/cgi-bin/basil.cgi
```

# Setup Apache to run Basil as CGI:

```

<IfModule mod_ssl.c>
<VirtualHost *:443>
        ServerName yobasic.com
        # Allow CGI under /cgi-bin/

        ScriptAlias /cgi-bin/ /usr/lib/cgi-bin/

        <Directory "/usr/lib/cgi-bin/">
            Options +ExecCGI
            AllowOverride None
            Require all granted
        </Directory>
      
        # Run any .basil files with basil.cgi
        
        AddHandler basil-script .basil
        Action basil-script /cgi-bin/basil.cgi

        # Alternatively, map URL like /app/foo.basil -> sets SCRIPT_FILENAME to /var/www/app/foo.basil
        # RewriteCond %{REQUEST_FILENAME} !-f
        # RewriteRule ^/app/(.+\.basil)$ /cgi-bin/basil.cgi [QSA, PT, E=SCRIPT_FILENAME:/var/www/app/$1]

        # Make sure CGI sees the real file path
        RewriteEngine On
        RewriteCond %{HANDLER} =basil-script
        RewriteRule ^ - [E=SCRIPT_FILENAME:%{REQUEST_FILENAME}]

        # Configure your Php Site like normal (example here is the Yore PHP framework)
        <Directory /var/www/yore/web>
            Options Indexes FollowSymLinks MultiViews
            AllowOverride All
            Require all granted
            RewriteEngine on
            RewriteCond %{REQUEST_FILENAME} !-f
            RewriteCond %{REQUEST_FILENAME} !-d
            RewriteRule ^(.*)$ /index.php/$1 [NC, L]
        </Directory>

        ServerAdmin webmaster@localhost
        DocumentRoot /var/www/yore/web

        ErrorLog ${APACHE_LOG_DIR}/error.log
        CustomLog ${APACHE_LOG_DIR}/access.log combined

# SSL configuration
SSLCertificateFile /etc/letsencrypt/live/yobasic.com-0002/fullchain.pem
SSLCertificateKeyFile /etc/letsencrypt/live/yobasic.com-0002/privkey.pem
Include /etc/letsencrypt/options-ssl-apache.conf
</VirtualHost>
</IfModule>


```



# 🌱 STATUS UPDATE _It's Working!!_
#### Prototype v0: tokens → Abstract Syntax Tree → bytecode → VM
The core is in place! We can now run simple programs with functions, recursion, locals, conditionals, and arithmetic.

See TODO.md for next steps.

See GOALS.md for the high-level vision.

See VISION.md for more details on language shape, stdlib, web story, tooling, performance, and roadmap.


# 🌱 STATUS UPDATE _It Compiles_ :clown_face:



# 🌱 HERE'S THE LATEST UPDATE :

# Basil (prototype v0)

Let's Grow some Basil! 🌿🌱🌱🌱


 
🍿 Quick Try:

```` 
# Using the "lex" command to see tokens:
cargo run -p basilc -- lex examples/hello.basil

output:
Print   'PRINT' @0..5
String  '"Hello, Basil!"'       @6..21
Semicolon       ';'     @21..22
Eof     ''      @24..24 
````

```` 
# Or the "run" command
cargo run -p basilc -- run examples/hello.basil

output:
"Hello, Basil!"
````


### (or with the basil puns like chop and sprout)

````
# See the tokens
cargo run -p basilc -- chop examples/hello.basil
# Run the file
cargo run -p basilc -- sprout examples/hello.basil
````

# see help

````
cargo run -p basilc -- --help
````

Output:
```
Basil CLI (prototype)

Commands (aliases in parentheses):
  init (seed)        Create a new Basil project
  run  (sprout)      Tokenize & run (v0: tokenize) a .basil file
  build (harvest)    Build project (stub)
  test (cultivate)   Run tests (stub)
  fmt  (prune)       Format sources (stub)
  add  (infuse)      Add dependency (stub)
  clean (compost)    Remove build artifacts (stub)
  dev  (steep)       Start dev mode (stub)
  serve (greenhouse) Serve local HTTP (stub)
  doc  (bouquet)     Generate docs (stub)
  lex  (chop)        Dump tokens from a .basil file (debug)
 
Usage:
  basilc <command> [args]

Examples:
  basilc run examples/hello.basil
  basilc sprout examples/hello.basil
  basilc init myapp
  

```



## 🍿 The Deep Geek Stuff:

We can make serve (greenhouse) spin up a tiny static file server for docs.


### 🐷 What's Done So Far:

+ Fill parser with the Pratt loop from the plan.
+ Implement a basil-bytecode Chunk and the VM dispatch loop.
+ Wire basilc to: lex → parse → compile → run.
+ Add examples: hello.basil, expr.basil, fib.basil.
+ Add CLI commands: run/sprout, lex/chop, help.
+ Most rudimentary BASIC features:
  - `PRINT` statement
  - `LET` for local variable declaration
  - Numeric literals and arithmetic expressions
  - Function declarations with `FUNC`/`RETURN`
  - Function calls with arguments
  - Recursion (e.g., Fibonacci)
  - `IF/THEN[/ELSE]` conditionals
  - Local variables and parameters
  - Comparison operators: `==`, `!=`, `<`, `<=`, `>`, `>=`
  - Stack-based bytecode VM with call frames
+ Basic error handling (panics on runtime errors for now).

#### 🐷 That's All Folks ! ! ! 


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

> Later: `emit_c/`, `emit_wasm/`, `bridge_napi/`, `bridge_hpy/`, `ffi_c_abi/`.

---

## 2) 🌱 Language subset / Extended Backus-Naur Form (EBNF)

```
program     := { declaration } EOF ;

declaration := "FUNC" ident "(" [parameters] ")" [":" type] block
             | "LET" ident [":" type] "=" expression ";"
             | statement ;

parameters  := ident [":" type] { ", " ident [":" type] } ;

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

arguments   := expression { ", " expression } ;

type        := IDENT ; // placeholder for v0, optional annotations only
```

---

## 3) 🌱 Tokens (v0)

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

## 4) 🌱 Pratt parser outline (binding powers)

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

## 5) 🌱 Abstract Syntax Tree (AST) (v0)

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

## 6) 🌱 Bytecode format v0 (stack‑based)

**Why stack first?**

* Easiest to emit from Abstract Syntax Tree.
* Minimal VM loop; great for bootstrapping.
* We can later add a register/SSA (Static Single Assignment) Intermediate Representation

and keep this as a portable baseline.

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

## 7) 🌱 Values & stack frames

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

## 8) 🌱 Minimal VM loop (Rust)

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

            ADD => bin_num(self, |a, b| a+b)?, 
            SUB => bin_num(self, |a, b| a-b)?, 
            MUL => bin_num(self, |a, b| a*b)?, 
            DIV => bin_num(self, |a, b| a/b)?, 
            NEG => { let a = as_num(self.pop())?; self.stack.push(Value::Num(-a)); }

            EQ => bin_cmp(self, |a, b| a==b)?, 
            NE => bin_cmp(self, |a, b| a!=b)?, 
            LT => bin_num_cmp(self, |a, b| a<b)?, 
            LE => bin_num_cmp(self, |a, b| a<=b)?, 
            GT => bin_num_cmp(self, |a, b| a>b)?, 
            GE => bin_num_cmp(self, |a, b| a>=b)?, 

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
fn bin_num<F: Fn(f64, f64)->f64>(vm: &mut VM, f: F) -> Result<(), VMError> {
    let b = as_num(vm.pop())?; let a = as_num(vm.pop())?;
    vm.stack.push(Value::Num(f(a, b))); Ok(())
}
fn bin_cmp<F: Fn(&Value, &Value)->bool>(vm: &mut VM, f: F) -> Result<(), VMError> {
    let b = vm.pop(); let a = vm.pop();
    vm.stack.push(Value::Bool(f(&a, &b))); Ok(())
}
```

---

## 9) 🌱 Compiler (Abstract Syntax Tree → bytecode) — essentials

### 9.1 🌱 Expression emission

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

### 9.2 🌱 Control flow (patching)

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

## 10) 🌱 CLI behavior (v0)

* `basilc run examples/expr.basil` → lex/parse/compile/execute.
* `basilc repl` → interactive (line → compile → run frame).
* `basilc dump --tokens file` | `--ast` | `--bc` → debugging.

---

## 11) 🌱 Example programs

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

## 12) 🌱 Testing strategy

* Unit tests per crate (lexer, parser, compiler, vm).
* Golden tests: source → bytecode hex dump → compare.
* E2E: run examples and verify stdout.

---

## 13) 🌱 Roadmap from here

1. **Check in skeleton**: crates, opcodes, minimal lexer, numeric literals, string interner.
2. **Implement Pratt parser** and statements `LET/IF/WHILE/RETURN/BLOCK`.
3. **Bytecode compiler** with patching for jumps; functions + call frames.
4. **Builtins**: `PRINT`, `clock()`; error reporting with spans.
5. **Release v0.1** with examples + docs.
6. **Next**: Booleans short‑circuit, arrays/maps, for‑loops, file I/O, import/module system.
7. **Then**: async runtime scaffold; C‑ABI & WASI plugin MVP; Postgres driver.
8. **Finally**: register/Static Single Assignment / Intermediate Representation and C/WASM/JS emitters.

---

## 14) 🌱 Licensing & contribution

* MIT or Apache‑2.0 for maximum adoption.
* `CONTRIBUTING.md` with rustfmt + clippy gates; CI on Windows/Linux.

---
 

🐖🐷🌿💻🎉🚀


# AOT compiler (bcc)

A new AOT compiler is available as a separate binary: `bcc`.
It transpiles Basil source to a tiny Rust crate and invokes Cargo to build a native executable.

Quickstart:

```
bcc aot examples/hello.basil --emit-project generated/hello
# or build directly (requires Rust toolchain):
bcc aot examples/hello.basil
```

See docs/compiler/AOT_COMPILER.md for full details, CLI flags, feature handling and templates.
