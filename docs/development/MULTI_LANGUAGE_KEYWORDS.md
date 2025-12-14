Keep **one canonical core language** (English-ish keywords, stable grammar), while letting people bolt on **“skins”** (localized keywords, personal slang, classroom dialects) without forking the compiler/interpreter.

### What I’d recommend architecturally

#### 1) Make `#lang` select a **preprocessor plugin**, not a different compiler

Think of it as: “before lexing/parsing, run this transform.”

* `#lang en` → identity (no changes)
* `#lang es` → rewrites `SI` → `IF`, `ENTONCES` → `THEN`, etc.
* `#lang pirate` → rewrites `YARR` → `PRINT` (lol)

This is safer than trying to “load a new grammar,” because grammars multiply complexity fast.

#### 2) Keep it **token-based**, not raw string replace

You want to rewrite at the *token* level so you don’t accidentally rewrite inside strings/comments:

✅ `SI x = 1 ENTONCES` → `IF x = 1 THEN`
❌ `PRINT "SI"` shouldn’t become `PRINT "IF"`

So the pipeline becomes:

**source → tokenize → preprocess tokens → parse → compile/run**

This also keeps “dialects” predictable.

---

## What should a `#lang` file look like?

### Option A: Simple mapping file (best first step)

Example `lang/es.map`:

```
SI=IF
ENTONCES=THEN
SINO=ELSE
FIN=END
IMPRIMIR=PRINT
```

Then your preprocessor:

* uppercase match for keywords (optionally case-insensitive)
* only replaces tokens of type `Identifier/KeywordCandidate`
* can support multiword phrases later (`FIN_SI` → `ENDIF` or `END IF`)

### Option B: A tiny DSL with rules (more power)

Example:

```
keyword SI -> IF
keyword ENTONCES -> THEN
keyword FIN_SI -> END IF
```

Later you can add:

* aliases by version (Basil vs BASIC differences)
* warnings (“deprecated keyword”)
* “soft” keywords (only keyword in certain contexts)

### Option C: Scriptable preprocessor (most power, more risk)

Let users write a preprocessor in Basil (or Lua/JS), but this can become a sandbox/security problem, and it’s harder to keep deterministic.

If you go here, I’d still start with A/B and only later add “script mode.”

---

## Key design decisions (so it doesn’t become chaos)

### Canonical core: always one truth

Even if users write Spanish Basil, internally you translate to canonical Basil tokens before parsing. That means:

* error messages can optionally show *both* (“expected THEN (ENTONCES)”)
* libraries/docs stay stable
* you don’t fork the ecosystem

### Scope: start with **keyword aliasing only**

At first, limit `#lang` to:

* keywords
* built-in function names (optional)
* maybe operator words (`AND`, `OR`, `NOT`)

Avoid changing:

* expression grammar
* precedence
* statement forms

Because as soon as dialects change grammar, you’ll get “this program works in pirate-lang but not in regular Basil” and support becomes a mess.

### Determinism + caching

Make preprocessing deterministic and cacheable:

* preprocessor file hash + source hash → transformed token stream hash
* nice for YoBASIC in the browser too

---

## How it could work across your 3 implementations

### Basil (Rust) and BASIC (Rust)

* Put a **shared preprocessor crate** used by both, e.g. `basil_preproc`
* It takes:

    * `Vec<Token>`
    * `LangProfile` (loaded from map/DSL)
    * returns `Vec<Token>` (or a wrapper with mapping metadata)

**Metadata is useful**:

* keep a map from transformed token positions back to original source spans
* so diagnostics point to the user’s source, not the canonical rewrite

### YoBASIC (Basic.js)

Same pipeline:

* lexer → `applyLangProfile(tokens)` → parser
* keep span mapping so runtime errors highlight the right place in the editor

Because you already have an IDE-ish environment, you can make this *feel magical*:

* dropdown: “Language: English / Español / Français / Pirate”
* show a “compiled canonical view” panel optionally

---

## Practical gotchas (worth solving early)

1. **Collisions**
   If someone maps `TO=THEN` you might break `FOR i = 1 TO 10`.
   Solution: keyword mappings should only replace **recognized keyword tokens**, not general identifiers, and you might need context for a few ambiguous tokens.

2. **Case + Unicode**
   Decide:

* are keywords case-insensitive?
* do you uppercase internally using Unicode casefolding?
  For non-English alphabets, you’ll want Unicode-safe casefolding.

3. **Multi-token rewrites**
   Some mappings are 1→2 tokens (`FIN_SI` → `END` `IF`). Your preprocessor should support expanding a token into a short token list.

4. **Error messages**
   Best UX: show canonical expectation plus the local alias.
   Example:
   “Expected `THEN` (or `ENTONCES` in `#lang es`).”

---

## My suggestion: a clean “v1” spec

* Directive: `#lang <name>` must appear before any code (except comments/blank lines)
* Lookup order:

    1. built-in lang profiles (`en`, `es`, `pt`, etc.)
    2. project-local path like `./lang/<name>.map` (or `./lang/<name>.dsl`)
* Format: simple `KEY=VALUE` lines, comments start with `;` or `#`
* Replacement operates on tokens of category: `IdentifierOrKeyword`
* Supports:

    * `A=B` (single token)
    * `A=B C` (expands to multiple tokens)

That’s enough to prove the concept across Basil/BASIC/YoBASIC and keep it sane.

---

If you want, I can write you a **Junie prompt** that:

* adds the `#lang` directive handling to Basil (Rust)
* adds the map-file loader + token-transform stage
* adds span mapping so diagnostics remain correct
* mirrors the same in Basic.js (YoBASIC)

…and keeps the shared parts in a reusable crate/module so you don’t duplicate logic.
