# RENDER$ template rendering and Fred directives

This guide introduces the RENDER$ built-in function and the lightweight “Fred” directive language available inside templates. It shows the syntax, supported directives, rules, and examples, and outlines future extensions.

Audience: Basil users who want to create simple view templates for CGI/web or other output generation.


## Overview

RENDER$(template$ [, context@]) interprets the input string as a template and returns a string result. It supports two kinds of embedded logic:

- Expression interpolation using double braces: {{ expr }} — where expr is a normal Basil expression evaluated at render time.
- Fred directives using @NAME(args) — inspired by Laravel Blade and the “Fred” syntax described in the design notes. Some directives are inline (direct value insertion), while others control flow (conditional blocks and simple case selection).


## Quick examples

- Variables and expressions:
  - Input: "Hello, {{ name$ }}! Today is {{ DATE$() }}." → If name$ = "Erik", outputs: Hello, Erik! Today is 2025-12-07.

- Directives inline:
  - "@URLENCODE(name$)" → Inserts the URL-encoded form of name$.
  - "@LEFT(title$, 10)" → Inserts the first 10 characters of title$.

- Flow control blocks:
  - @IF(cond)
    ... text when true ...
    @ELSE
    ... text when false ...
    @ENDIF

  - @CASE(cond1)
    ... when cond1 is truthy ...
    @CASE(cond2)
    ... when cond2 is truthy ...
    @ENDCASE

- Includes:
  - "@INCLUDE("partials/header.html")" reads a file (relative to the script directory by default), renders it, and inserts the result.


## Exact behaviors

### {{ expr }} interpolation

- expr is a normal Basil expression. It is evaluated during rendering.
- Whitespace inside braces is ignored: {{  A$  }} is valid.
- Result is converted to string using Basil’s normal rules for PRINT/concatenation.

Notes:
- v1 evaluates expressions in the context of global variables and user-defined functions. Additionally, when you pass an optional context dictionary, its keys are made available as variables during render time (see below). Locals from the caller are not automatically visible.
- Nested Fred calls are allowed inside expressions by using @NAME(...) inside expr; they are evaluated first and replaced with string literals in the expression.


### Fred directive syntax

- Form: @NAME(arg1, arg2, ...)
- Names are case-insensitive.
- Arguments are Basil expressions separated by commas. They can include string literals, function calls, and other Fred direct calls. Flow-control directives are statements and not valid as arguments.

Two directive families:

1) Direct evaluation directives — inline replacement with computed value.
2) Flow control directives — conditional inclusion of template blocks.


### Supported directives (v1)

Direct evaluation:
- @LEFT(s$, n%) → first n characters of s$ (Unicode-aware).
- @RIGHT(s$, n%) → last n characters of s$.
- @MID(s$, start%, len%) → substring starting at 1-based start% for len% characters.
- @TRIM(s$) → s$ with leading/trailing whitespace removed.
- @URLENCODE(s$) → application/x-www-form-urlencoded encoding.
- @ENV(name$) → environment variable value or "null" if missing.
- @SERVER(name$) → server/environment variable; same behavior as ENV in v1.
- @REQUEST(name$) → first matching request parameter (POST or GET) value, or "null".
- @SESSION(name$) → session value or "null" (stub in v1; may be enhanced in CGI context later).
- @INCLUDE(path$) → reads file contents (relative to script directory if path is relative), renders them recursively, inserts result. Nonexistent file yields "null".

Flow control:
- @IF(cond) ... @ELSE ... @ENDIF — cond truthiness follows Basil rules: nonzero numbers and nonempty strings are true; null is false.
- @CASE(condA) ... @CASE(condB) ... @ENDCASE — evaluates each cond in order; renders the first true block. No default arm in v1.

User-defined function fallback:
- If @NAME is not a built-in directive, Basil looks for a user-defined function NAME and calls it with the evaluated arguments. The return value is inserted via normal string conversion.


### Nesting and composition

- You can nest directives and interpolations.
- You can use direct Fred calls inside expressions (e.g., {{ @LEFT(A$, 2) + "..." }}).
- Flow-control directives can contain other directives and interpolations.


### Escaping

- A single @ not followed by a letter or underscore is treated as literal text (e.g., email like user@example.com does not start a directive at the @e...).
- For literal {{, consider breaking it across pieces (e.g., {{ "{" }}{) or avoiding the sequence. A dedicated escape may be added later.


### Include safety

- Includes are depth-limited (default: 16 levels) to avoid infinite recursion. Circular includes will error out once the depth limit is exceeded.


## Examples

1) Simple greeting:

LET name$ = "Erik"
LET tpl$ = "Hello, {{ name$ }}!"
PRINT RENDER$(tpl$)

→ Hello, Erik!

2) Inline directive:

LET a$ = "abcdefg"
PRINT RENDER$("Start: @LEFT(a$, 3) :End")

→ Start: abc :End

3) If/Else block:

LET a% = 2
LET tpl$ = "@IF(a% = 1)one@ELSEother@ENDIF"
PRINT RENDER$(tpl$)

→ other

4) Case block:

LET x% = 0
LET tpl$ = "@CASE(x%=1)one@CASE(x%=2)two@CASE(x%=0)zero@ENDCASE"
PRINT RENDER$(tpl$)

→ zero

5) Include a partial (relative to the running script file):

PRINT RENDER$("@INCLUDE(\"partials/header.html\")\nBody here\n@INCLUDE(\"partials/footer.html\")")

6) Optional context dictionary (Option 2):

LET pet@ = { "name": "Fido", "species": "dog", "age": 7, "weight": 15.0 }
LET view$ = """
Pet name: {{ name$ }}
Species: {{ species$ }}
Age: {{ age% }}
Weight: {{ weight }}
Greeting: Hello, {{ name$ }}! You are a good {{ species$ }}.
Left 2 of name: @LEFT(name$, 2)
"""
PRINT RENDER$(view$, pet@)

→ This works because the context dictionary injects variables:
- Always the exact key (case-insensitive global name match).
- If the value is a String, it also injects key$ (unless the key already ends with $).
- If the value is an Integer, it also injects key% (unless the key already ends with %).
- Other types (Float, Bool, Dict, List, Array, Object) use just key without an added suffix.

Precedence: context variables override same-named module globals for the duration of the render. Caller locals/params are NOT automatically visible.


## Error handling

- Template parse errors and evaluation errors include a short message. In v1, rendering fails fast with an error. A future safe mode can optionally embed visible placeholders instead.


## Performance notes

- Rendering parses the template to a small in-memory representation and evaluates pieces as needed. Expressions are evaluated via a child VM seeded with the parent’s globals and functions for correctness. This is simple and correct; optimizations can be added later.


## Future extensions

These are ideas that keep the rules simple yet useful and can be added incrementally:

- Raw vs escaped output: add {{{ expr }}} for raw and keep {{ … }} to mean HTML-escaped (or vice versa). Start with no auto-escaping.
- Shorthand @ELSEIF(cond) instead of @ELSE … @IF(cond).
- Looping: @FOREACH(item, collection) … @ENDFOREACH to iterate lists/arrays/dicts.
- Layouts/sections: @EXTENDS("layout"), @SECTION("name") … @ENDSECTION, @YIELD("name").
- Template comments: {{-- … --}} or @COMMENT … @ENDCOMMENT to strip content from output.
- Safer mode: a RENDER_SAFE$ variant or an optional second parameter mode% that disables UDF fallback, @INCLUDE, or {{ … }}.
- Caching: memoize parsed template AST or rendered results by content hash.
- Locals visibility: extend scope capture to include local variables from the caller’s frame.


## Notes

- This feature is available in Basil only; the sister Basic project remains unchanged.
- Behavior for REQUEST/SERVER/SESSION may vary by runtime environment (CGI vs console). In non-CGI, REQUEST and SESSION typically return "null".
