### Prompt for Junie Ultimate — Implement list auto-extend, shorthands, and [] indexing in the BASIC sister project

Context for Junie Ultimate
- Target repo: the BASIC sister project of “basil”. It is nearly identical in core language/runtime but does NOT include the compiler, web server, or extended features. Assume an interpreter/VM-centric codebase.
- Goal: Implement the exact feature set we completed in the basil project during a prior session, with the same semantics and guardrails.

What to implement (high-level)
1) Lists auto-extend on indexed SET with gap-fill always ON (no directive/flag). GET out-of-range still errors.
2) Shorthand operators and sugars:
   - x += y and x -= y for bare variables (numbers: arithmetic; strings: concatenation).
   - Postfix x++ and x-- for bare variables (equivalent to x += 1 and x -= 1).
   - List append: list@ += value and list@[] = value (both forms append one element).
   - Dict merge/overwrite: dict@ += {"k": v, ...} merges keys, overwriting existing ones.
   - Do NOT support arr += v (arrays remain fixed-size; no REDIM/REDIM PRESERVE in scope).
3) Indexing syntax and semantics:
   - Arrays: allow both parentheses and square brackets for element indexing: a%(i) and a%[i] are equivalent.
   - Lists and dictionaries: use square brackets for indexing: list@[i], dict@["key"].
   - External/indexing convention is 1-based for both arrays and lists. Enforce consistently across all code paths.
   - Arrays are a distinct, fixed-size type; lists can grow. No auto-extend for arrays.

Detailed requirements and notes
- List auto-extend (gap-fill always on):
  - Writing to any 1-based index > current length extends the list to that index, filling missing slots with Null.
  - Example: DIM l@ = [1,2]; l@[5] = 9 results in [1,2,null,null,9].
  - Reading out of range still throws an error.

- Shorthands:
  - += / -= on bare identifiers only (e.g., i% += 1, s$ += "x"). These lower to simple assignment: i% = i% + 1; s$ = s$ + "x".
  - ++ / -- postfix on bare identifiers (i%++, n--). Not supported on indexed expressions.
  - List append:
    - list@ += value → append value to list.
    - list@[] = value → syntactic sugar for appending to end.
  - Dict merge:
    - dict@ += {"a":1, "b":2} merges keys, overwriting duplicates.
  - Arrays: do not add arr += v. Keep arrays immutable in size.

- Indexing:
  - Arrays: support both [] and () in the interpreter/VM for GET and SET, in all contexts (top-level and inside procedures). Bounds-check and do not extend.
  - Lists/dicts: use [] consistently. Lists auto-extend on SET; dicts overwrite on key SET.
  - Keep visible indices 1-based everywhere. If you use 0-based internally, convert at the boundary and ensure no code path leaks 0-based behavior.

Troubles we hit before (and how to avoid them here)
1) Compound assignment on indexed targets parsed as "+" then unexpected "=":
   - Symptom: code like a%(i) += 1 produced “unexpected token Assign”.
   - Fix/future-proofing: restrict += / -= and ++ / -- to bare identifiers only. For elements use explicit assignment: a%(i) = a%(i) + 1.

2) Using [] for array SET routed to list/dict handler:
   - Symptom: a%[i] = v caused runtime error “Attempted [] on a non-list/dict value”.
   - Root cause: The [] path for SET was compiled/evaluated via the list/dict index-set routine instead of the array element setter in certain scopes.
   - How to avoid: In all interpreter/VM paths, dispatch [] on arrays to the array element SET handler, not the list/dict handler. Do this uniformly for both GET and SET, in every scope.

3) Index base mismatch between different code paths:
   - Symptom: some [] paths treated indices as 0-based while others used 1-based, causing off-by-one issues.
   - How to avoid: Standardize on 1-based indices externally for arrays and lists. If arrays are 0-based internally, subtract 1 on entry and add checks accordingly for both GET and SET, regardless of whether the target was accessed with [] or (). Add tests to catch regressions.

4) Mixing bracket rules for lists:
   - In basil, lists/dicts are accessed with []. If your sister project historically used () for lists, pick one convention and normalize in the parser. For parity with basil, prefer [] for lists/dicts and support both [] and () only for arrays.

Scope boundaries (explicitly out of scope here)
- Do not introduce REDIM or REDIM PRESERVE.
- Do not add arr += v.
- Do not add ++/-- or += / -= to indexed expressions (e.g., a[i]++); those remain explicit assignments.
- No compiler or web/extended features—make changes in the interpreter/VM and parser only.

Implementation checklist (interpreter/VM-centric sister project)
- Parser changes:
  - Accept list@[] = expr and lower to: set list@[LEN(list@) + 1] = expr.
  - Add compound assignments for bare identifiers: name += expr, name -= expr.
  - Add postfix ++ and -- for bare identifiers: name++, name--.
  - For dict merge: name@ += { key: value, ... } → expand to multiple key sets or emit a merge helper.
  - Accept [] as an alternative to () for array indexing in both GET and SET.

- Interpreter/VM changes:
  - Index GET/SET dispatcher for [] should handle three types distinctly:
    - List: 1-based; SET auto-extends with Null gap-fill; GET bounds-checks.
    - Dict: keys must be strings; GET/SET as usual (SET overwrites existing key).
    - Array: support both [] and (); 1-based externally; bounds-check; no auto-extend.
  - For arithmetic/string shorthands on bare identifiers: implement as desugared assignments at parse/AST stage or as simple runtime sugar.
  - For list append (name@ += v): either desugar to name@[LEN(name@)+1] = v or implement a small built-in/primitive LIST_APPEND that takes (list, value).
  - For dict merge (name@ += { ... }): either desugar into per-key SETs or a DICT_MERGE primitive.

Tests to add (minimal):
- Lists
  - DIM l@ = [1,2]; l@[4] = 9; PRINT LN LEN(l@) → 4; and l@[3] is null.
  - l@ += 5; l@[] = 6; contents end with 5,6.
- Dicts
  - DIM d@ = {"a":1}; d@ += {"a":2, "b":3}; expect {"a":2, "b":3}.
- Arrays
  - DIM a%(3); a%[1] = 10; a%(2) = 20; PRINTLN a%[1], a%(2) → 10,20.
  - Attempt a%[4] = 1 should error (no auto-extend).
- Shorthands
  - i% = 5; i%++; PRINTLN i% → 6. i% -= 2; PRINTLN i% → 4.
  - s$ = "Hi"; s$ += "!" → "Hi!".
- Negative tests
  - a%[1] += 1 should be rejected or produce a clear error pointing to “compound ops only on variables”.

Acceptance criteria
- All tests above pass with consistent 1-based indexing semantics.
- Lists always gap-fill with Null on SET to an index beyond current length; arrays never auto-extend.
- [] works for arrays in both GET and SET across all scopes/contexts; lists/dicts use [].
- += / -= and ++ / -- work for bare identifiers; list append and dict merge shorthands behave as specified.
- Error messages are clear for misuse (e.g., trying arr += v, or compound ops on indexed expressions).

Examples (quick sanity):
```
DIM mixed@ = [1, "two", 3]
mixed@ += "four"
mixed@[] = 5               ' [1, "two", 3, "four", 5]
mixed@[LEN(mixed@)+2] = 9  ' gap-fills with null

DIM d@ = {"a": 1}
d@ += {"b": 2, "a": 3}   ' → {"a":3, "b":2}

DIM a%(3)
FOR i% = 1 TO LEN(a%)
    a%[i%] = i% * 10
NEXT
PRINTLN a%(1), a%[2], a%[3]  ' 10 20 30

DIM i% = 5: i%++: PRINTLN i%  ' 6
DIM s$ = "Hi": s$ += "!": PRINTLN s$  ' Hi!
```

Deliverables
- Parser and interpreter/VM changes per above.
- A short README note documenting the new syntax and semantics.
- Example scripts covering lists, dicts, arrays, and shorthands.

Please proceed with these changes in the BASIC sister project, keeping the code paths simple and uniform (no compiler assumptions). Prioritize consistent 1-based indexing and the correct dispatch for [] on arrays vs. lists/dicts to avoid the pitfalls we encountered.
