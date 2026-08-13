# Junie Ultimate Prompt — Basil Day 2: Define AtomicBASIC Shared IR Contracts

We are beginning work on **AtomicBASIC**, a restricted BASIC profile intended for simple interactive games running inside the AtomicFlix Roku application.

The long-term architecture is:

```text
AtomicBASIC Source
        |
        v
Basil Compiler
        |
        v
AtomicFlix Game IR / Bytecode
       / \
      /   \
 Roku VM  Browser VM
```

For the current proof-of-concept, the AtomicFlix.com PHP project will temporarily hand-author or generate JSON Game IR, and the Roku application will interpret that JSON.

The Basil project is **not yet responsible for compiling AtomicBASIC source**.

This Day 2 task is intentionally limited to establishing Basil-side data structures, validation, fixtures, and a clean future compilation-target boundary.

Do not build the AtomicBASIC compiler yet.

---

# Existing Basil Project Context

This is an existing Rust workspace implementing the Basil BASIC language and related tools.

The workspace contains components approximately like:

```text
basilcore/
    common/
    lexer/
    ast/
    parser/
    bytecode/
    compiler/
    vm/

basilc/
bcc/
basil-serve/
basil-wasm/
```

There may be additional crates or structural differences in the current checkout.

Before changing anything, inspect the actual workspace and follow existing architectural conventions instead of forcing this exact structure.

The purpose of this task is to add a **small, isolated AtomicBASIC contract layer** without disrupting normal Basil compilation or execution.

---

# Overall Goal

Implement the **Day 2 — Define the Shared Contracts** portion of our AtomicBASIC proof-of-concept roadmap found in "docs/ROKU_GAME_POC_7_DAYS.md" as well as other ROKU* markdown documents in this folder.

At the end of this task, Basil should:

1. Define Rust types representing **AtomicFlix Game Manifest v1**.
2. Define Rust types representing **AtomicFlix Game IR v1**.
3. Define the initial AtomicBASIC VM operation set.
4. Define the initial VM execution-state set.
5. Define the runtime-error contract.
6. Deserialize valid JSON fixtures into Rust structures.
7. Serialize those structures back into equivalent JSON.
8. Validate important structural properties of Game IR.
9. Contain tests for valid and invalid fixtures.
10. Have a clean placeholder concept for an `AtomicBasic` compilation target.
11. Leave all existing Basil functionality unchanged.

This is a contract and foundation task, not an interpreter/compiler implementation task.

---

# Important Scope Restrictions

Please do **not**:

- Implement an AtomicBASIC parser.
- Change the existing Basil grammar.
- Change ordinary Basil semantics.
- Implement Roku bytecode.
- Implement a browser VM.
- Implement a new VM.
- Implement SceneGraph concepts.
- Add AtomicBASIC built-ins to Basil.
- Implement source-to-IR compilation.
- Build a sophisticated optimizer.
- Refactor unrelated Basil code.
- Perform broad dependency upgrades.
- Break public APIs unnecessarily.
- Introduce a large abstraction framework for what is currently a small contract module.

Keep the change small, isolated, tested, and future-facing.

---

# Step 1 — Inspect and Restore the Workspace

Before implementing AtomicBASIC changes:

1. Inspect the Cargo workspace.
2. Identify where shared serializable data structures normally belong.
3. Identify whether `serde` and `serde_json` are already workspace dependencies.
4. Identify how existing compiler targets, output formats, or feature profiles are represented.
5. Run:

```text
cargo build --workspace
cargo test --workspace
```

If the workspace currently has unrelated failures, document them clearly.

Do not spend this task performing broad cleanup unless a small fix is necessary to allow the new code and tests to build.

Also inspect:

```text
basilcore/
basilc/
Cargo.toml
```

and any existing common/shared crate that would be the natural home for cross-runtime contract structures.

---

# Step 2 — Choose a Small AtomicBASIC Module Location

Prefer placing the shared contract types inside `basilcore` or its most appropriate shared crate.

A possible structure is:

```text
basilcore/
    atomic/
        mod.rs
        manifest.rs
        ir.rs
        error.rs
        validation.rs
```

However, follow the actual crate structure.

If `basilcore` is a workspace directory containing multiple crates rather than one crate, choose the crate that best represents common compiler/runtime data.

Avoid creating a brand-new crate unless the existing architecture strongly suggests that is appropriate.

The code should be reusable later by:

- `basilc`
- a future AtomicBASIC compiler
- `basil-wasm`
- tests
- possible future tooling

---

# Step 3 — Implement AtomicFlix Game Manifest v1 Types

Represent this JSON contract:

```json
{
  "format": "atomicflix-game",
  "formatVersion": 1,
  "runtimeVersion": 1,
  "gameId": "haunted-door",
  "revision": 1,
  "title": "Haunted Door",
  "programUrl": "/api/atomic-games/program-v1.json",
  "posterUrl": "/api/atomic-games/assets/haunted-door.jpg"
}
```

Create a Rust structure conceptually similar to:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtomicGameManifest {
    pub format: String,

    #[serde(rename = "formatVersion")]
    pub format_version: u32,

    #[serde(rename = "runtimeVersion")]
    pub runtime_version: u32,

    #[serde(rename = "gameId")]
    pub game_id: String,

    pub revision: u64,

    pub title: String,

    #[serde(rename = "programUrl")]
    pub program_url: String,

    #[serde(rename = "posterUrl")]
    pub poster_url: Option<String>,
}
```

Use idiomatic Rust field names while preserving the exact external JSON names through Serde.

Define constants for the initial contract where appropriate:

```rust
pub const ATOMIC_GAME_FORMAT: &str = "atomicflix-game";
pub const ATOMIC_GAME_FORMAT_VERSION: u32 = 1;
pub const ATOMIC_GAME_RUNTIME_VERSION: u32 = 1;
```

Do not hardcode these values throughout the implementation.

---

# Step 4 — Implement AtomicFlix Game IR v1 Types

Represent the top-level structure:

```json
{
  "format": "atomicflix-game-ir",
  "version": 1,
  "entryPoint": "start",
  "instructions": []
}
```

Create a type conceptually similar to:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AtomicGameProgram {
    pub format: String,
    pub version: u32,

    #[serde(rename = "entryPoint")]
    pub entry_point: String,

    pub instructions: Vec<AtomicInstruction>,
}
```

Define:

```rust
pub const ATOMIC_GAME_IR_FORMAT: &str = "atomicflix-game-ir";
pub const ATOMIC_GAME_IR_VERSION: u32 = 1;
```

---

# Step 5 — Model the Instruction Set

The initial operation set contains exactly these 13 operations:

```text
LABEL
LAYOUT
IMAGE
TITLE
TEXT
CHOICE
WAITCHOICE
GOTO
END
SET
ADD
IF_EQ
WAITKEY
```

Prefer a strongly typed enum rather than storing arbitrary operation strings everywhere.

A tagged Serde enum is acceptable if it produces the required JSON shape.

For example, conceptually:

```rust
#[serde(tag = "op")]
pub enum AtomicInstruction {
    #[serde(rename = "LABEL")]
    Label {
        name: String,
        line: u32,
    },

    #[serde(rename = "LAYOUT")]
    Layout {
        value: String,
        line: u32,
    },

    ...
}
```

The serialized JSON must remain compatible with fixtures such as:

```json
{
  "op": "LABEL",
  "name": "start",
  "line": 1
}
```

and:

```json
{
  "op": "CHOICE",
  "text": "Open the door",
  "target": "openDoor",
  "line": 7
}
```

Do not invent a different JSON structure merely because another Rust representation would be prettier.

The cross-platform JSON contract is authoritative.

---

# Step 6 — Define Each Instruction Shape

Implement the following instruction contracts.

## LABEL

```json
{
  "op": "LABEL",
  "name": "start",
  "line": 1
}
```

Fields:

```text
name: string
line: positive source line number
```

---

## LAYOUT

```json
{
  "op": "LAYOUT",
  "value": "fullscreen_text",
  "line": 2
}
```

Fields:

```text
value: string
line: source line
```

Do not enforce a complete layout enum yet unless it is trivial.

For this proof-of-concept, `fullscreen_text` is the known layout.

---

## IMAGE

```json
{
  "op": "IMAGE",
  "value": "/api/atomic-games/assets/haunted-door.jpg",
  "line": 3
}
```

---

## TITLE

```json
{
  "op": "TITLE",
  "value": "The Old House",
  "line": 4
}
```

---

## TEXT

```json
{
  "op": "TEXT",
  "value": "The front door is slightly open.",
  "line": 5
}
```

---

## CHOICE

```json
{
  "op": "CHOICE",
  "text": "Open the door",
  "target": "openDoor",
  "line": 7
}
```

Fields:

```text
text
target
line
```

---

## WAITCHOICE

```json
{
  "op": "WAITCHOICE",
  "line": 9
}
```

---

## GOTO

```json
{
  "op": "GOTO",
  "target": "finish",
  "line": 12
}
```

---

## END

```json
{
  "op": "END",
  "line": 20
}
```

---

## SET

```json
{
  "op": "SET",
  "variable": "score",
  "value": 0,
  "line": 3
}
```

For `value`, use a representation capable of handling:

- string
- number
- Boolean
- null

Prefer `serde_json::Value` for this first contract unless the existing Basil architecture already has a simple reusable value representation that serializes cleanly to ordinary JSON.

Do not introduce a complex VM value system just for this.

---

## ADD

```json
{
  "op": "ADD",
  "variable": "score",
  "value": 1,
  "line": 10
}
```

For now, represent `value` in a way compatible with JSON numbers.

Validation may require this to be numeric.

---

## IF_EQ

```json
{
  "op": "IF_EQ",
  "variable": "score",
  "value": 1,
  "target": "winner",
  "line": 15
}
```

Fields:

```text
variable
value
target
line
```

---

## WAITKEY

```json
{
  "op": "WAITKEY",
  "variable": "lastKey",
  "line": 14
}
```

The `variable` should ideally be optional so future code can wait for a key without storing it.

For example:

```rust
variable: Option<String>
```

If omitted during serialization, use Serde's normal optional-field handling.

---

# Step 7 — Add Useful Instruction Helpers

Provide small helpers if they improve validation and tooling.

For example:

```rust
impl AtomicInstruction {
    pub fn op_name(&self) -> &'static str;
    pub fn source_line(&self) -> u32;
    pub fn jump_target(&self) -> Option<&str>;
}
```

`jump_target()` should return a target for instructions that reference labels:

- `CHOICE`
- `GOTO`
- `IF_EQ`

Do not overgeneralize.

These helpers will later be useful to:

- compiler diagnostics
- validation
- optimizer passes
- Roku/browser compatibility tooling

---

# Step 8 — Define VM State Contract

Add an enum representing these exact initial states:

```text
READY
RUNNING
WAITING_FOR_CHOICE
WAITING_FOR_KEY
FINISHED
ERROR
```

For example:

```rust
pub enum AtomicVmState {
    Ready,
    Running,
    WaitingForChoice,
    WaitingForKey,
    Finished,
    Error,
}
```

If serialized to JSON or displayed externally, use the exact uppercase contract strings:

```text
READY
RUNNING
WAITING_FOR_CHOICE
WAITING_FOR_KEY
FINISHED
ERROR
```

Also add comments noting future likely states:

```text
WAITING_FOR_TIMER
WAITING_FOR_API
WAITING_FOR_MEDIA
```

Do not implement those future states now.

---

# Step 9 — Define Runtime Error Contract

Represent:

```json
{
  "type": "runtime_error",
  "message": "Unknown label: basement",
  "instruction": 12,
  "sourceLine": 18
}
```

Create a structure conceptually like:

```rust
pub struct AtomicRuntimeError {
    #[serde(rename = "type")]
    pub error_type: String,

    pub message: String,

    pub instruction: usize,

    #[serde(rename = "sourceLine")]
    pub source_line: u32,
}
```

Define:

```rust
pub const ATOMIC_RUNTIME_ERROR_TYPE: &str = "runtime_error";
```

Use a **zero-based instruction index**.

Use a **one-based source line number**.

Document those conventions clearly.

Do not build a complete error hierarchy yet.

---

# Step 10 — Implement Game IR Validation

Add a small validator.

Conceptually:

```rust
pub fn validate_atomic_program(
    program: &AtomicGameProgram
) -> Result<(), Vec<AtomicValidationError>>
```

Use whatever result/error style best matches the project.

Validation should check at minimum:

### Program-level validation

- `format == "atomicflix-game-ir"`
- `version == 1`
- `entryPoint` is non-empty
- instructions are present
- entry point corresponds to an existing `LABEL`

### Instruction validation

- Every source line is greater than zero.
- Label names are non-empty.
- Label names are unique.
- All targets from:
  - `CHOICE`
  - `GOTO`
  - `IF_EQ`

  refer to labels that exist.
- `SET.variable` is non-empty.
- `ADD.variable` is non-empty.
- `ADD.value` is numeric.
- `IF_EQ.variable` is non-empty.
- `WAITKEY.variable`, when present, is non-empty.
- `CHOICE.text` is non-empty.
- `CHOICE.target` is non-empty.

Do not attempt to perform control-flow analysis yet.

Do not detect unreachable code yet.

Do not optimize anything yet.

---

# Step 11 — Define Validation Errors

Create a clear validation-error type.

It may resemble:

```rust
pub struct AtomicValidationError {
    pub message: String,
    pub instruction: Option<usize>,
    pub source_line: Option<u32>,
}
```

If useful, add an enum for validation error kinds, but do not overbuild this.

Validation errors should produce useful human-readable messages such as:

```text
Entry point 'start' does not exist.
Instruction 7 references unknown label 'basement'.
Duplicate label 'start'.
ADD requires a numeric value.
```

This validation layer is expected to become useful later for:

- `basilc`
- linting
- compiler diagnostics
- editor feedback
- test tooling

---

# Step 12 — Add Canonical Fixtures

Create test fixtures in a logical location such as:

```text
tests/fixtures/atomic/
```

or within the appropriate crate.

Add at minimum:

```text
manifest-v1.json
program-v1.json
runtime-error-v1.json
program-invalid-missing-label.json
program-invalid-unknown-op.json
```

If a tagged enum causes `serde_json` to reject an unknown operation during deserialization, that is fine.

In that case, the unknown-op fixture should test deserialization failure rather than validation failure.

---

# Canonical Valid Program Fixture

Use a coherent fixture equivalent to:

```json
{
  "format": "atomicflix-game-ir",
  "version": 1,
  "entryPoint": "start",
  "instructions": [
    {
      "op": "LABEL",
      "name": "start",
      "line": 1
    },
    {
      "op": "LAYOUT",
      "value": "fullscreen_text",
      "line": 2
    },
    {
      "op": "IMAGE",
      "value": "/api/atomic-games/assets/haunted-door.jpg",
      "line": 3
    },
    {
      "op": "TITLE",
      "value": "The Old House",
      "line": 4
    },
    {
      "op": "TEXT",
      "value": "The front door is slightly open.",
      "line": 5
    },
    {
      "op": "CHOICE",
      "text": "Open the door",
      "target": "openDoor",
      "line": 7
    },
    {
      "op": "CHOICE",
      "text": "Run away",
      "target": "finish",
      "line": 8
    },
    {
      "op": "WAITCHOICE",
      "line": 9
    },
    {
      "op": "LABEL",
      "name": "openDoor",
      "line": 11
    },
    {
      "op": "TEXT",
      "value": "The door opens by itself.",
      "line": 12
    },
    {
      "op": "SET",
      "variable": "score",
      "value": 0,
      "line": 13
    },
    {
      "op": "ADD",
      "variable": "score",
      "value": 1,
      "line": 14
    },
    {
      "op": "IF_EQ",
      "variable": "score",
      "value": 1,
      "target": "survived",
      "line": 15
    },
    {
      "op": "GOTO",
      "target": "finish",
      "line": 16
    },
    {
      "op": "LABEL",
      "name": "survived",
      "line": 18
    },
    {
      "op": "TEXT",
      "value": "You survived the doorway.",
      "line": 19
    },
    {
      "op": "WAITKEY",
      "variable": "lastKey",
      "line": 20
    },
    {
      "op": "GOTO",
      "target": "finish",
      "line": 21
    },
    {
      "op": "LABEL",
      "name": "finish",
      "line": 23
    },
    {
      "op": "TITLE",
      "value": "The End",
      "line": 24
    },
    {
      "op": "TEXT",
      "value": "Thanks for playing.",
      "line": 25
    },
    {
      "op": "END",
      "line": 26
    }
  ]
}
```

Make sure the Rust fixture and type definitions agree exactly with this shared contract.

---

# Step 13 — Add Serialization/Deserialization Tests

Add tests verifying:

## Manifest

- Valid manifest deserializes.
- `formatVersion` maps to the Rust `format_version` field.
- `runtimeVersion` maps correctly.
- `gameId` maps correctly.
- `programUrl` maps correctly.
- `posterUrl` maps correctly.
- Re-serialization uses camelCase external names.

## Program

- Valid Game IR deserializes.
- All 13 operation variants deserialize successfully.
- Entry point is `start`.
- Instruction count is as expected.
- Re-serialization produces equivalent logical JSON.

Avoid testing pretty-print whitespace or object-key ordering.

Compare parsed `serde_json::Value` values if useful.

## Runtime Error

- Runtime-error fixture deserializes.
- `sourceLine` maps correctly.
- Instruction index remains zero-based.

---

# Step 14 — Add Validation Tests

Add tests for at least:

### Valid program

Expected:

```text
validation succeeds
```

### Missing entry-point label

Example:

```text
entryPoint = "missing"
```

Expected validation failure.

### Unknown jump target

Example:

```json
{
  "op": "GOTO",
  "target": "basement",
  "line": 10
}
```

Expected validation failure.

### Duplicate labels

Expected validation failure.

### Invalid ADD value

Example:

```json
{
  "op": "ADD",
  "variable": "score",
  "value": "banana",
  "line": 5
}
```

Expected validation failure.

### Unknown operation

Expected deserialization failure if using a strict enum.

---

# Step 15 — Introduce a Future Compilation Target

Inspect how `basilc` currently represents compilation modes, targets, backends, or output formats.

Add the smallest clean placeholder for an AtomicBASIC target.

Conceptually:

```rust
pub enum CompilationTarget {
    Basil,
    AtomicBasic,
}
```

Do **not** force this exact enum into the codebase if an existing target abstraction already exists.

Follow the project's conventions.

The target should be recognizable by future tooling as:

```text
atomic
```

or:

```text
atomicbasic
```

Prefer:

```text
atomic
```

for CLI-facing shorthand if that matches existing naming conventions.

If adding CLI parsing now would require extensive unrelated changes, it is acceptable to define the target internally only.

The primary requirement is to establish a clean place for a future AtomicBASIC compiler backend.

---

# Step 16 — Optional Minimal CLI Stub

Only if this fits naturally and requires little code, allow something like:

```text
basilc --target atomic somefile.atomic
```

For Day 2 it should **not compile the file**.

It may return a clear message such as:

```text
AtomicBASIC compilation target is registered but not implemented yet.
```

This should be considered a successful and intentional stub behavior, not a panic.

If implementing this CLI option requires invasive work, defer it.

The contract types and validation are the required work.

---

# Step 17 — Add Documentation

Create or update:

```text
docs/ATOMIC_BASIC.md
```

or the closest existing documentation location.

Document:

## Purpose

AtomicBASIC is a restricted Basil-related language profile intended for interactive AtomicFlix games.

## Current Phase

Basil currently defines:

- Manifest contract
- Game IR contract
- VM operation names
- VM state names
- Runtime-error structure
- Validation
- Future target placeholder

Basil does **not yet** compile AtomicBASIC source.

## Architecture

```text
AtomicBASIC Source
        |
        v
Basil Compiler               <-- future
        |
        v
AtomicFlix Game IR
       / \
      /   \
 Roku VM  Browser VM
```

## Initial Operations

List all 13:

```text
LABEL
LAYOUT
IMAGE
TITLE
TEXT
CHOICE
WAITCHOICE
GOTO
END
SET
ADD
IF_EQ
WAITKEY
```

## Indexing Convention

Explicitly document:

```text
instruction indexes: zero-based
source line numbers: one-based
```

## Versioning

Document:

```text
Manifest format version: 1
Game IR version: 1
Runtime version: 1
```

---

# Step 18 — Preserve Existing Basil Behavior

Before finishing, run:

```text
cargo fmt --all
cargo build --workspace
cargo test --workspace
```

If practical without unrelated cleanup:

```text
cargo clippy --workspace
```

Confirm that:

- Existing Basil compilation still works.
- Existing Basil VM behavior is unchanged.
- Existing tests remain green except for clearly pre-existing failures.
- New AtomicBASIC tests pass.
- No warnings were introduced unnecessarily.

If the workspace already contains failures that existed before these changes, document them separately rather than hiding them.

---

# Suggested Internal API

Do not force this exact API if the project has better conventions, but something close to this would be useful:

```rust
use basilcore::atomic::{
    AtomicGameManifest,
    AtomicGameProgram,
    AtomicInstruction,
    AtomicRuntimeError,
    AtomicVmState,
    validate_atomic_program,
};
```

The eventual compiler should be able to construct an `AtomicGameProgram` directly without serializing intermediary ad-hoc objects.

---

# Acceptance Criteria

Do not consider the task complete until all applicable items below are satisfied.

- [ ] Existing Basil workspace structure has been inspected.
- [ ] Existing workspace build/test state has been recorded.
- [ ] An AtomicBASIC/shared Atomic game contract module exists.
- [ ] Manifest v1 has a Rust representation.
- [ ] Game IR v1 has a Rust representation.
- [ ] JSON external field names match the cross-platform contract.
- [ ] All 13 initial instruction operations are represented:
  - `LABEL`
  - `LAYOUT`
  - `IMAGE`
  - `TITLE`
  - `TEXT`
  - `CHOICE`
  - `WAITCHOICE`
  - `GOTO`
  - `END`
  - `SET`
  - `ADD`
  - `IF_EQ`
  - `WAITKEY`
- [ ] VM states are represented:
  - `READY`
  - `RUNNING`
  - `WAITING_FOR_CHOICE`
  - `WAITING_FOR_KEY`
  - `FINISHED`
  - `ERROR`
- [ ] Runtime-error v1 has a Rust representation.
- [ ] Instruction indexes are documented as zero-based.
- [ ] Source lines are documented as one-based.
- [ ] Valid manifest fixture deserializes.
- [ ] Valid program fixture deserializes.
- [ ] Valid runtime-error fixture deserializes.
- [ ] Valid structures serialize back to compatible JSON.
- [ ] Entry-point validation exists.
- [ ] Label-target validation exists.
- [ ] Duplicate-label validation exists.
- [ ] Basic `ADD` numeric validation exists.
- [ ] Unknown operation handling is tested.
- [ ] A future AtomicBASIC compilation target placeholder exists if it fits cleanly.
- [ ] Existing Basil behavior is unchanged.
- [ ] New tests pass.
- [ ] Workspace builds successfully, aside from clearly documented pre-existing issues.
- [ ] AtomicBASIC architecture documentation exists.

---

# Implementation Philosophy

Please make reasonable architectural choices based on the existing Basil codebase.

Prefer:

- idiomatic Rust
- Serde-compatible structures
- strict contract versioning
- readable validation errors
- small modules
- good unit tests
- minimal coupling
- future compiler reuse

Avoid:

- speculative abstractions
- premature bytecode design
- compiler implementation
- unrelated refactoring
- broad dependency upgrades
- changing Basil semantics

The key objective for Day 2 is simple:

> **Make Basil understand, represent, serialize, deserialize, and validate the exact same AtomicFlix Game IR contract that the AtomicFlix website and Roku runtime will use later.**

At this stage, Basil is establishing the language/tooling side of the bridge. It is not yet responsible for driving traffic across it.