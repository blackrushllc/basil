### Environment Variables
Yes. Basil includes built‑in support for working with environment variables:
- `ENV$(name$)` — function to read an environment variable
- `SETENV NAME = value;` — statement to set a process‑local environment variable
- `EXPORTENV NAME = value;` — statement to set and attempt to persist an environment variable for future processes
- `LOADENV%(filename$?)` — function to load name=value pairs from a file (defaults to ".env") into the current process environment

Related process control that commonly interacts with env vars:
- `SHELL "...";` — run a command (inherits your current env)
- `EXIT n;` — exit with code `n`

---

### `ENV$` — read an environment variable
- Type: function, returns `String`.
- Behavior: returns the value if set, otherwise an empty string `""`.
- Usage:
```basil
PRINTLN "PATH=", ENV$("PATH");
LET tmp$ = ENV$("TMPDIR");
IF LEN(tmp$) == 0 THEN LET tmp$ = ENV$("TEMP");
```

### `SETENV` — set a process‑local environment variable
- Type: statement.
- Behavior: sets the variable for the current Basil process (and any child processes you start from Basil, e.g., via `SHELL`). It does not modify the parent shell’s environment.
- The right‑hand side can be a quoted string, number, boolean, or scalar variable; it is stored as a string.
- Usage:
```basil
SETENV DEMO_VAR = "42";
PRINTLN ENV$("DEMO_VAR");  ' -> 42
```

### `EXPORTENV` — set and try to persist for future processes
- Type: statement.
- Behavior: does everything `SETENV` does, and also attempts to persist the value for future processes:
  - On Windows: uses `setx` under the hood. The new value becomes visible to newly started shells/applications, not the already running parent shell.
  - On non‑Windows: persistence to the parent shell is generally not possible; Basil still sets the process‑local value so children see it.
- Usage:
```basil
EXPORTENV DEMO_EXPORT = "HELLO WORLD";
PRINTLN ENV$("DEMO_EXPORT");
```

### `LOADENV%` — load environment variables from a file
- Type: function, returns `Integer` (1 = success, 0 = error reading file).
- Behavior: reads a text file of `NAME=VALUE` pairs. Lines starting with `#` or `;` are comments; blank lines are ignored. Sets variables for the current process; malformed lines produce warnings but do not fail the call.
- If no filename is provided, or it is blank, defaults to `.env` in the current directory.
- Usage:
```basil
IF LOADENV%() THEN PRINTLN "Loaded .env"; ELSE PRINTLN "No .env file";
PRINTLN "API_KEY=", ENV$("API_KEY");
```

### Related: `SHELL` and `EXIT`
- `SHELL "cmd";` executes the string in the system shell and waits for completion. Child processes inherit any variables you set with `SETENV`/`EXPORTENV` during this run.
```basil
SHELL "cmd /C echo Value is %DEMO_VAR%";
```
- `EXIT n;` exits the interpreter with code `n` (default `0`). Handy when using env vars to signal status to calling scripts.

### Practical examples
```basil
PRINTLN "USERNAME:", ENV$("USERNAME");
PRINTLN "PATH (prefix):", LEFT$(ENV$("PATH"), 60), "...";

' Process-local set (visible to this Basil run and its children)
SETENV APP_MODE = "dev";
PRINTLN "APP_MODE=", ENV$("APP_MODE");

' Persist for future shells where supported (Windows best effort via SETX)
EXPORTENV MY_TOOL_HOME = "C:\\Tools\\MyTool";

' Use the variable in a child process
SHELL "cmd /C echo MY_TOOL_HOME is %MY_TOOL_HOME%";

' Exit with a code based on presence
IF LEN(ENV$("REQUIRED_KEY")) == 0 THEN EXIT 2; ELSE EXIT 0;
```

### Notes and gotchas
- All environment values are strings. Convert as needed, e.g., `VAL(ENV$("PORT"))`.
- Missing variables return `""`; test with `LEN(...) == 0`.
- `SETENV` affects only the current Basil process and its children; your already‑open shell will not see the change.
- `EXPORTENV` on Windows uses `setx`, which has practical limits and may truncate very long values; it also only affects new shells started after the change.
- To modify `PATH` for child processes during this run:
```basil
SETENV PATH = ENV$("PATH") + ";C:\\MyBin";
```
- Common envs you might read in web/CGI contexts include `REQUEST_METHOD`, `QUERY_STRING`, and `HTTP_COOKIE` via `ENV$()`.

