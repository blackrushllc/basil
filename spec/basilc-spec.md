Title: Basilc Core Language Specification (Scaffold)
Status: Draft (Scaffold)
Version: 0.1.0-draft
Date: 2025-11-06

Note on scope (normative): This specification defines the Basilc core interpreted language and its interpreter behavior. It explicitly EXCLUDES all add-on object libraries found under basil-objects and basil-objects-* folders, as well as ancillary products (bcc, basil-serve, basilica). Implementations claiming conformance to this document MUST implement only the core language and environment defined herein.

Normative language conventions: The key words “MUST”, “MUST NOT”, “REQUIRED”, “SHALL”, “SHALL NOT”, “SHOULD”, “SHOULD NOT”, “RECOMMENDED”, “NOT RECOMMENDED”, “MAY”, and “OPTIONAL” in this document are to be interpreted as described in RFC 2119/BCP 14.

Change log (informative):
- 0.1.0-draft: Initial scaffold created from core source overview (basilc + basilcore/*).

1. Introduction and Conformance
1.1 Purpose and Non-goals (informative)
- Purpose: Provide an implementation-neutral description of the Basilc core language sufficient to develop a compatible interpreter in other host languages (e.g., C++, Java, JavaScript).
- Non-goals: Define or require any add-on object libraries (basil-objects*) or ancillary tools (bcc, basil-serve, basilica); specify non-core host integration details.

1.2 Document Structure (informative)
- Sections marked normative contain requirements for conformance. Others are informative guidance.

1.3 Conformance Classes (normative)
- Class A: Core Interpreter. An implementation that reads Basil source text, produces diagnostics, and executes programs according to this spec.
- Class B: Core Parser/Analyzer. An implementation that reads Basil source text and produces a conformant AST and diagnostics, without execution.
- Class C: Embedding Interface. A host that embeds an interpreter exposing a minimal set of functions (initialize, evaluate, set/get variables, capture diagnostics) with the semantics defined in Section 10.3.

1.4 Versioning (informative)
- Spec uses Semantic Versioning (MAJOR.MINOR.PATCH). Language changes that break programs increment MAJOR; additive increments MINOR; fixes PATCH.

2. Design Overview (informative)
2.1 Philosophy
- A BASIC-inspired, readable language with structured control flow and optional legacy constructs for migration (e.g., GOTO/GOSUB, LABEL), while encouraging structured forms (IF/WHILE/FOR/SELECT CASE/TRY).

2.2 Execution Model
- Direct interpretation over a compiled bytecode or IR (implementation detail). Semantics herein MUST be preserved regardless of underlying representation.
- Deterministic execution order unless explicitly marked implementation-defined.

2.3 Minimal Core Environment
- Side effects limited to deterministic console output and process exit codes. No network or host-specific objects are required for core. File I/O beyond printing MAY be reserved for future core revisions or left to add-ons.

3. Source Text and Environment (normative)
3.1 Character Encoding
- Source files MUST be UTF-8. The interpreter MAY normalize CRLF/CR to LF internally.

3.2 Lines, Newlines, and Statement Terminators
- Newlines and semicolons act as statement separators. The lexer/parser MAY implement newline insertion consistent with Section 4.4 and 5.

3.3 Comments
- Single-quote (') to end of line, REM to end of line, and // to end of line are comments. Comments are ignored except for their role in separating tokens.

3.4 Modules and Files
- Core defines a single-compilation-unit program. Module systems and inclusion are out of scope for the core.

4. Lexical Grammar (normative)
4.1 Tokens and Punctuation
- Delimiters: ( ) { } [ ] , ; : .
- Operators: + - * / MOD (or %) = == != < <= > >=
- Literals: numeric, string
- Identifiers

4.2 Keywords (reserved words)
- Control: IF, THEN, ELSE, WHILE, DO, BEGIN, END, SELECT, CASE, IS, FOR, TO, STEP, NEXT, EACH, IN, FOREACH, ENDFOR
- Functions: FUNC, RETURN
- Boolean/Null: TRUE, FALSE, NULL
- Logical: AND, OR, NOT
- Flow: BREAK, CONTINUE, GOTO, GOSUB, LABEL
- Error handling: TRY, CATCH, FINALLY, RAISE
- I/O and process: PRINT, PRINTLN, EXIT, STOP, SHELL, EXEC, EVAL
- Declarations: LET, DIM, AS, TYPE, CLASS, NEW, DESCRIBE, AUTHOR
- Environment: SETENV, EXPORTENV

Note: Some keywords listed above MAY be reserved for compatibility though their semantics are not required in the minimal core. The definitive set of required constructs is listed in Section 7 and 9. Keywords outside the required set MUST be treated as reserved identifiers and produce diagnostics if used as identifiers.

4.3 Identifiers
- Start: letter or underscore; Continue: letters, digits, underscore. Unicode identifier rules are TBD for core 0.1.0.

4.4 Numeric Literals
- Integers and reals (decimal). Hex/bin literals are TBD. Exact ranges and overflow behavior are defined in 7.2.

4.5 String Literals and Escapes
- Double-quoted strings. Supported escapes: TBD (at minimum \n, \r, \t, \", \\). The exact escape set SHALL be specified in a future draft; unknown escapes SHOULD be a lexical error.

4.6 Tokenization Rules
- Longest match applies. Whitespace and comments separate tokens. Newline insertion rules MAY treat a newline as Semicolon unless line continuation applies (e.g., trailing operators or explicit continuation with underscore). Exact rules TBD; see Appendix A notes.

4.7 Lexical Errors
- An invalid character or malformed literal MUST produce a diagnostic including location and a stable error code (see Section 8).

5. Syntactic Grammar (normative)
Note: The following grammar is a scaffold derived from the current basilcore parser. Final grammar MUST be provided as machine-readable EBNF in spec/grammar/basilc.ebnf.

5.1 High-level
- Program ::= { LineDirective | Statement }
- Statement ::= Block | IfStmt | WhileStmt | ForStmt | SelectCaseStmt | TryStmt | FuncDecl | ReturnStmt | BreakStmt | ContinueStmt | LetStmt | DimStmt | PrintStmt | GotoStmt | LabelStmt | ExprStmt

5.2 Expressions (precedence/associativity)
- Unary: NOT, unary -
- Multiplicative: * / MOD
- Additive: + -
- Relational: < <= > >=
- Equality: == !=
- Logical: AND OR (short-circuit)
- Assignment: identifier = Expression (right-associative)

5.3 Provisional EBNF Snippet (to be validated)
Expression     = Assignment ;
Assignment     = OrExpr | Identifier, '=', Assignment ;
OrExpr         = AndExpr, { 'OR', AndExpr } ;
AndExpr        = Equality, { 'AND', Equality } ;
Equality       = Relational, { ( '==' | '!=' ), Relational } ;
Relational     = Additive, { ( '<' | '<=' | '>' | '>=' ), Additive } ;
Additive       = Multiplicative, { ( '+' | '-' ), Multiplicative } ;
Multiplicative = Unary, { ( '*' | '/' | 'MOD' ), Unary } ;
Unary          = ( 'NOT' | '-' ), Unary | Primary ;
Primary        = Number | String | Identifier | 'TRUE' | 'FALSE' | 'NULL' | '(', Expression, ')' ;

IfStmt     = 'IF', '(', Expression, ')', Statement, [ 'ELSE', Statement ]
           | 'IF', Expression, 'THEN', Statement, [ 'ELSE', Statement ] ;
WhileStmt  = 'WHILE', '(', Expression, ')', Statement
           | 'WHILE', Expression, [ 'DO' ], Statement ;
ForStmt    = 'FOR', Identifier, '=', Expression, 'TO', Expression, [ 'STEP', Expression ], Statement, 'NEXT' ;
SelectCaseStmt = 'SELECT', 'CASE', Expression, SelectBody ;
SelectBody = '{', { CaseArm }, [ CaseElse ], '}'
           | { CaseArm }, [ CaseElse ], 'END', [ 'SELECT' ] ;
CaseArm    = 'CASE', CasePatterns, StatementSeq ;
CasePatterns = CasePattern, { ',', CasePattern } ;
CasePattern = 'IS', RelOp, Expression
            | Expression, [ 'TO', Expression ] ;
RelOp      = '==' | '=' | '!=' | '<' | '<=' | '>' | '>=' ;
TryStmt    = 'TRY', StatementSeq, 'CATCH', [ Identifier ], StatementSeq, [ 'FINALLY', StatementSeq ], 'END' ;
FuncDecl   = 'FUNC', Identifier, '(', [ ParamList ], ')', StatementSeq, 'END' ;
ParamList  = Identifier, { ',', Identifier } ;
ReturnStmt = 'RETURN', [ Expression ] ;
BreakStmt  = 'BREAK' ;
ContinueStmt = 'CONTINUE' ;
LetStmt    = 'LET', Identifier, '=', Expression ;
DimStmt    = 'DIM', Identifier, [ 'AS', TypeName ] ;
PrintStmt  = ( 'PRINT' | 'PRINTLN' ), [ PrintArgs ] ;
PrintArgs  = Expression, { ',', Expression } ;
GotoStmt   = 'GOTO', Identifier | 'GOSUB', Identifier ;
LabelStmt  = 'LABEL', Identifier ;
StatementSeq = '{', { Statement }, '}' | Statement, { Statement } ;
ExprStmt   = Expression, [ ';' ] ;

Note: The exact grammar MUST be adjusted to match parser behavior, including optional braces and newline-as-semicolon rules.

5.4 Disambiguation
- Dangling ELSE associates with nearest IF not already having an ELSE.
- Lookahead may be used to distinguish label declarations vs identifiers in expressions.

6. Static Semantics (normative)
6.1 Scope and Binding
- Lexical scoping within functions and blocks. Global scope at program top-level. Shadowing rules: TBD.

6.2 Declarations
- LET introduces/assigns variables in the nearest scope. DIM MAY declare names (types TBD). FUNC declares a function.

6.3 Types
- Core is dynamically typed with runtime values defined in 7.2. Static type annotations (AS, TYPE, CLASS) are reserved and MAY be no-ops or diagnostics in core 0.1.0; to be finalized.

6.4 Errors
- Refer to Section 8 for classification. Name resolution errors MUST report use-before-declaration where applicable (TBD specifics).

7. Runtime Semantics (normative)
7.1 Evaluation Order
- Left-to-right evaluation of operands. Logical AND/OR are short-circuiting.

7.2 Values and Runtime Types
- Numbers (at least IEEE-754 double), Strings (UTF-8), Booleans, Null, Arrays/Records TBD for core 0.1.0.
- Numeric conversions and string coercions: TBD. Division by zero behavior MUST produce a runtime error unless otherwise specified.

7.3 Control Flow
- IF/ELSE conditional execution.
- WHILE loops with optional DO and block forms.
- FOR loops with TO and optional STEP; 'NEXT' terminates the loop.
- SELECT CASE with value, range (lo TO hi), and comparator arms (CASE IS <op> <expr>), with optional CASE ELSE.
- TRY/CATCH/FINALLY with RAISE to signal exceptions.
- BREAK/CONTINUE affect nearest enclosing loop.
- GOTO/GOSUB and LABEL are supported for legacy control flow; interaction with structured blocks MUST be defined (TBD).

7.4 Functions
- FUNC defines a function; RETURN exits the current function optionally returning a value; parameters are passed by value (TBD pass-by semantics).

7.5 I/O and Side Effects
- PRINT writes textual representations to standard output. PRINTLN appends a newline. Output encoding MUST be UTF-8.
- EXIT and STOP terminate the program with an exit code (see Section 8.5). SHELL/EXEC/EVAL are reserved; their core behavior is TBD or MAY be excluded from minimal core.

7.6 Determinism and Time
- Core execution MUST be deterministic given the same inputs and environment.

7.7 Implementation-Defined Aspects
- Numeric overflow handling, string concatenation vs addition rules, and exact truthiness conventions MUST be documented by each implementation if not fully specified here (TBD in future draft).

8. Error Model and Diagnostics (normative)
8.1 Classes of Errors
- Lexical, Syntax, Static (binding/validity), Runtime.

8.2 Recovery Policy
- Implementations MAY use limited recovery for syntax errors (e.g., panic-mode to next statement boundary). If recovery is implemented, diagnostics MUST not cascade misleadingly.

8.3 Diagnostic Structure
- Each diagnostic MUST include: message, stable code, severity, and location (line, column, offset). A machine-readable JSON form is RECOMMENDED (Appendix C, TBD).

8.4 Exceptions and RAISE
- RAISE produces a runtime exception with a message/value (TBD). TRY/CATCH/FINALLY semantics MUST specify propagation and finalization order (TBD details).

8.5 Exit Behavior
- Exit codes: 0 success; 1 runtime error; 2 syntax/static error; 64 usage error.

9. Standard Core Environment (normative)
9.1 Built-ins
- print(x1, x2, ...): PRINT/PRINTLN statements. Exact formatting, separators (space vs comma behavior), and newline behavior MUST be specified (TBD; default: space-separated; PRINTLN adds trailing newline).

9.2 Environment Interaction
- SETENV/EXPORTENV core semantics are TBD. In the minimal core class, these MAY be unsupported and should produce a diagnostic if used.

9.3 Determinism
- Built-ins MUST be deterministic and side-effect limited to described behavior.

10. Interpreter Interface (normative)
10.1 CLI Invocation (basilc)
- basilc [OPTIONS] [--] <input>
- OPTIONS (provisional; align with current basilc behavior):
  - -e, --eval <code>       Evaluate inline source and print result/output.
  - -c, --check             Parse and report diagnostics only (no execution).
  - -o, --out <file>        Write diagnostics/output to file (UTF-8).
  - --json-diag             Emit diagnostics using diagnostic.schema.json (TBD).
  - -v, --version           Print version and exit.
  - -h, --help              Show help and exit.
- EXIT CODES: See 8.5.

10.2 REPL (if provided)
- Prompt, multiline rules, and interrupt handling TBD. If unsupported, basilc SHOULD return usage.

10.3 Embedding Interface (abstract)
- initialize(config): Create an interpreter instance
- evaluate(source, options): Returns result/diagnostics
- set(name, value) / get(name): Host variable exchange
- on_diagnostic(callback): Receive diagnostics
- The above define semantics only; host FFI specifics are out of scope.

10.4 Module Loading
- Out of scope for the core 0.1.0.

11. Conformance (normative)
11.1 Core-Conformant Implementation
- MUST implement Sections 3–8 fully and provide CLI behaviors in Section 10.1.

11.2 Test Suite
- Implementations SHOULD pass the reference suite: positive (programs + expected outputs), negative (programs + expected diagnostics), structural (programs + expected AST). The suite is authoritative.

11.3 Canonical Artifacts
- Grammar EBNF (spec/grammar/basilc.ebnf), AST schema (spec/schemas/ast.schema.json), diagnostic schema (spec/schemas/diagnostic.schema.json). These are normative when published.

12. Security and Resource Constraints (informative)
- Implementations SHOULD provide execution time and memory limits. Sandboxing guidance TBD.

13. Rationale and Notes (informative)
- Selected constructs (e.g., SELECT CASE with ranges and comparators) reflect existing parser support in basilcore.
- Legacy constructs (GOTO/GOSUB/LABEL) included for compatibility but discouraged in new code.

Appendices
A. Full Grammar (normative)
- Will contain the consolidated EBNF. Source of truth for syntax once published. TODO.

B. Canonical AST Specification (normative)
- JSON Schema for AST nodes to support tooling interop. TODO.

C. Diagnostic JSON Schema (normative)
- JSON Schema for machine-readable diagnostics (Problem Details compatible). TODO.

D. Examples (informative)
- Simple programs with expected outputs and failing cases with expected diagnostics. TODO.

E. Glossary (informative)
- Definitions of terms and notations used in the spec. TODO.

Scope cross-check (informative):
- In scope (core): basilc (CLI), basilcore/common, basilcore/lexer, basilcore/ast, basilcore/parser, basilcore/bytecode, basilcore/compiler, basilcore/vm.
- Out of scope (add-ons): basil-objects, basil-objects-aws, basil-objects-net, basil-objects-crypto, basil-objects-sql, basil-objects-orm, and any similar basil-objects-* folders.
- Out of scope (ancillary): bcc, basil-serve, basilica, and related installer/distribution tooling.

Editorial TODOs for next draft
- Validate lexical escapes against current lexer behavior; document precisely.
- Align precedence/associativity and assignment semantics with parser.
- Decide on required vs reserved keywords list; remove or mark non-core.
- Specify numeric ranges/overflow and division semantics.
- Specify PRINT/PRINTLN formatting and argument rules.
- Provide machine-readable artifacts and seed conformance tests.
