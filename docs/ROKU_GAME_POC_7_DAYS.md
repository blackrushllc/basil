This week should favor **visible integration over architectural perfection**. The Roku VM and PHP compiler can be
deliberately primitive, while the Rust work establishes a clean future bridge without becoming a dependency.

# AtomicBASIC Seven-Day Proof-of-Concept Roadmap

## End-of-Week Success Criterion

By the end of Day 7:

1. A developer opens a minimal AtomicBASIC editor on AtomicFlix.com.
2. The developer edits and saves a small game program.
3. AtomicFlix compiles the source into JSON-based AtomicFlix Game IR.
4. A side-loaded AtomicFlix Roku app loads the game through an API.
5. The BrightScript VM displays text and an image.
6. The VM stops for a menu selection from the Roku remote.
7. The VM branches to the appropriate scene and continues execution.
8. The developer changes the source on the website.
9. The developer selects **Reload Game** on Roku.
10. The changed behavior appears without rebuilding or side-loading the Roku app again.

The proof-of-concept should include two or three tiny demonstration programs, but none needs to resemble a finished
game.

---

# Proof-of-Concept Scope

## AtomicBASIC Commands

The first week should support only:

```basic
:start
LAYOUT "fullscreen_text"
IMAGE "https://atomicflix.com/path/house.jpg"
TITLE "The Old House"
TEXT "The front door is slightly open."

CHOICE "Open the door" openDoor
CHOICE "Run away" finish
WAITCHOICE

:openDoor
TEXT "The door opens by itself."
GOTO finish

:finish
TITLE "The End"
TEXT "You survived."
END
```

Optional logic commands, added only after the basic pipeline works:

```basic
SET score 0
ADD score 1
IF_EQ score 2 winner
GOTO loser
```

## Initial JSON Game IR

The PHP compiler can produce readable instructions:

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
      "value": "https://atomicflix.com/path/house.jpg",
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
    }
  ]
}
```

The `line` property will make Roku runtime errors traceable to the source program.

---

# Day 1 — Restore the Development Environments

## Objective

Verify that all three development surfaces build and run before adding AtomicBASIC functionality.

## AtomicFlix.com Tasks

* Create a feature branch such as:

```text
feature/atomicbasic-poc
```

* Start the local PHP application.
* Verify database connectivity.
* Verify authentication and the admin or user dashboard.
* Identify where the Roku source lives inside the project.
* Locate the existing Roku feed-loading and HTTP-request code.
* Locate an existing Roku screen that already displays:

    * A poster or image
    * Text
    * A selectable list
    * Remote-control focus
* Record the commands required to run:

    * PHP application
    * Front-end build, if applicable
    * Tests
    * Roku package build
* Add a temporary project note:

```text
docs/atomicbasic-poc.md
```

This file will contain URLs, test usernames, branch names, Roku IP information, and development commands.

## Roku Tasks

* Build the existing Roku project without changes.
* Side-load it onto the development Roku.
* Verify that the existing AtomicFlix app starts normally.
* Confirm that BrightScript console or telnet debugging works.
* Confirm that changes to a visible string can be rebuilt and side-loaded.
* Identify:

    * Main SceneGraph scene
    * Main menu controller
    * Network Task components
    * Existing dialog or keyboard components
    * Existing image and video components

## Basil Tasks

Create a separate branch:

```text
feature/atomicbasic-profile
```

Then:

* Run:

```text
cargo build --workspace
cargo test --workspace
```

* Record compilation failures, test failures, warnings, or outdated dependencies.
* Build the main executables individually:

    * `basilc`
    * `bcc`
    * `basil-serve`
* Run a known-good sample Basil program.
* Verify that RustRover recognizes the workspace and test configuration.
* Do not begin major dependency upgrades unless required to restore the build.

## Day 1 Exit Gate

* AtomicFlix.com runs locally.
* AtomicFlix can be side-loaded and opened on Roku.
* BrightScript debugging is available.
* Basil builds or has a clearly documented list of failures.
* Both projects have dedicated AtomicBASIC branches.
* The development commands are documented.

---

# Day 2 — Define the Shared Contracts

## Objective

Create one small specification that PHP, Rust, and BrightScript can all follow.

## Shared Design Tasks

Document:

### Game Manifest Version 1

```json
{
  "format": "atomicflix-game",
  "formatVersion": 1,
  "runtimeVersion": 1,
  "gameId": "haunted-door",
  "revision": 1,
  "title": "Haunted Door",
  "programUrl": "https://atomicflix.test/api/atomic-games/haunted-door/program",
  "posterUrl": "https://atomicflix.test/images/games/haunted-door.jpg"
}
```

### Initial VM Operations

Required:

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
```

Desirable during the week:

```text
SET
ADD
IF_EQ
WAITKEY
```

### VM States

```text
READY
RUNNING
WAITING_FOR_CHOICE
WAITING_FOR_KEY
FINISHED
ERROR
```

### Runtime Error Structure

```json
{
  "type": "runtime_error",
  "message": "Unknown label: basement",
  "instruction": 12,
  "sourceLine": 18
}
```

## AtomicFlix.com Tasks

* Add JSON schema examples or PHP fixtures under a location such as:

```text
tests/Fixtures/AtomicGames/
```

* Add:

    * `manifest-v1.json`
    * `program-v1.json`
    * `haunted-door.atomic`
* Decide on the API route prefix:

```text
/api/atomic-games/
```

## Roku Tasks

* Add matching development fixture files inside the Roku project.
* Create a small BrightScript function that:

    * Reads a bundled JSON program
    * Calls `ParseJson`
    * Prints the parsed structure to the debug console
* Do not execute the instructions yet.

## Basil Tasks

Add an AtomicBASIC module or namespace without changing normal Basil behavior.

Suggested structure:

```text
basilcore/
    atomic/
        mod.rs
        ir.rs
        manifest.rs
```

Create Rust structures corresponding to the shared format:

```rust
pub struct AtomicGameProgram {
    pub format: String,
    pub version: u32,
    pub entry_point: String,
    pub instructions: Vec<AtomicInstruction>,
}
```

Add Serde serialization and deserialization tests using the same JSON fixture.

Add an initial target enum or profile identifier:

```rust
pub enum CompilationTarget {
    Basil,
    AtomicBasic,
}
```

This target does not need to compile AtomicBASIC yet.

## Day 2 Exit Gate

* The manifest and IR formats are documented.
* PHP, Rust, and Roku each possess the same fixture.
* BrightScript can parse the fixture.
* Rust can deserialize and reserialize it.
* All teams and tools agree on operation names and field names.

---

# Day 3 — Build the Static Roku Game Screen

## Objective

Create the television interface before creating the VM.

## Roku Tasks

Add a hidden or development-only menu item:

```text
AtomicBASIC Sandbox
```

Create components resembling:

```text
components/games/
    AtomicGameScene.xml
    AtomicGameScene.brs
    AtomicGameMenu.xml
    AtomicGameMenu.brs
```

The game scene should support:

* Background image
* Title
* Body text
* Vertical choice menu
* Selected-choice focus indicator
* Back-button handling
* A development status label

Create public fields or helper functions such as:

```text
setLayout()
setImage()
setTitle()
setText()
setChoices()
clearChoices()
showError()
```

Initially, hardcode:

```text
The Old House

The front door is slightly open.

> Open the door
  Ring the bell
  Run away
```

Remote behavior:

* Up moves the selection upward.
* Down moves the selection downward.
* OK selects the option.
* Back exits to the AtomicFlix menu.

Print the selected option to the debug console.

## AtomicFlix.com Tasks

* Add the initial AtomicBASIC editor route and placeholder page.
* The page can initially contain:

    * Heading
    * Source textarea
    * Save button
    * Compile button
    * Placeholder compiler result panel
* Restrict access to an existing authenticated development user or administrator.
* No database table is required until Day 4.

## Basil Tasks

* Add one sample file:

```text
examples/atomic/haunted-door.atomic
```

* Add a placeholder CLI target:

```text
basilc --target atomic examples/atomic/haunted-door.atomic
```

For now, the command may:

* Validate that the file exists.
* Read the source.
* Print a clear message that AtomicBASIC compilation is not implemented.
* Exit cleanly with an intentional status.

Better still, allow:

```text
basilc --target atomic --emit-stub-ir
```

to emit the shared fixture IR.

## Day 3 Exit Gate

* AtomicFlix opens a static game screen on Roku.
* The remote can navigate and select choices.
* Back returns safely to AtomicFlix.
* The website exposes a protected placeholder editor.
* The Basil CLI recognizes the future AtomicBASIC target.

---

# Day 4 — Build the Website Data Model and Tiny Compiler

## Objective

Save a game program in AtomicFlix.com and compile the first small language into JSON IR.

## Database Tasks

Create one proof-of-concept table:

```text
atomic_games
```

Suggested fields:

```text
id
user_id
slug
title
source_code
compiled_ir
compile_status
compile_errors
revision
is_published
created_at
updated_at
```

Recommended types:

* `source_code`: LONGTEXT
* `compiled_ir`: LONGTEXT or JSON
* `compile_errors`: TEXT or JSON
* `revision`: unsigned integer, default 0
* `is_published`: Boolean

Use one game per user for the proof-of-concept:

```text
UNIQUE(user_id)
UNIQUE(slug)
```

## Controller Tasks

Add controller actions for:

```text
GET  /atomic-games/editor
POST /atomic-games/editor/save
POST /atomic-games/editor/compile
```

The editor should show the current user’s single game record.

## Tiny PHP Compiler

Create a deliberately line-oriented compiler:

```text
app/Services/AtomicGames/
    AtomicGameCompiler.php
    AtomicGameCompilationResult.php
    AtomicGameCompilationException.php
```

Compiler steps:

1. Split source into lines.
2. Ignore blank lines.
3. Ignore comment lines beginning with `'` or `REM`.
4. Recognize labels beginning with `:`.
5. Parse command names.
6. Parse quoted strings.
7. Validate argument counts.
8. Collect declared labels.
9. Validate jump and choice targets.
10. Emit instruction objects with source-line numbers.

Commands for Day 4:

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
```

A source label:

```basic
:start
```

compiles to:

```json
{
  "op": "LABEL",
  "name": "start",
  "line": 1
}
```

A choice:

```basic
CHOICE "Open the door" openDoor
```

compiles to:

```json
{
  "op": "CHOICE",
  "text": "Open the door",
  "target": "openDoor",
  "line": 8
}
```

## Editor Tasks

The editor should display:

* Title
* Slug
* Source textarea
* Save button
* Save and Compile button
* Revision
* Compile status
* Compilation errors
* Read-only compiled JSON panel

Increment `revision` only after successful compilation.

## Basil Tasks

* Add Rust IR validation methods:

    * Program format is recognized.
    * Version is supported.
    * Entry point exists.
    * All label targets exist.
* Add tests using:

    * Valid fixture
    * Missing-label fixture
    * Unknown-op fixture

The Rust project remains independent of the PHP compiler.

## Day 4 Exit Gate

* A user can save AtomicBASIC source.
* PHP compiles it into JSON IR.
* Errors identify the source line.
* Successful compilation increments the revision.
* Rust validates equivalent fixture programs.

---

# Day 5 — Build the BrightScript VM

## Objective

Execute a bundled instruction program on Roku.

## Roku VM Files

Suggested structure:

```text
source/games/
    AtomicGameVM.brs
    AtomicGameRenderer.brs
    AtomicGameDiagnostics.brs
```

The VM should maintain:

```text
program
instructions
instructionPointer
labels
variables
pendingChoices
status
lastError
```

## VM Initialization

When a program is loaded:

1. Validate the top-level object.
2. Read the instruction array.
3. Build a label-to-instruction-index map.
4. Locate the entry point.
5. Reset the renderer.
6. Begin execution.

## Instruction Execution

Implement:

### `LAYOUT`

Calls the renderer’s layout function.

### `IMAGE`

Sets the current image URL.

### `TITLE`

Sets title text.

### `TEXT`

Sets body text.

### `CHOICE`

Adds an item to `pendingChoices`.

### `WAITCHOICE`

* Sends the pending choices to the menu.
* Changes VM state to `WAITING_FOR_CHOICE`.
* Returns control to SceneGraph.

### `GOTO`

Moves the instruction pointer to the target label.

### `END`

Changes status to `FINISHED`.

### `LABEL`

Performs no visible action during execution.

## Execution Slicing

Use a function such as:

```text
runSlice(maxInstructions)
```

Set a small initial limit, such as 100 or 500 instructions.

When the limit is reached:

* Yield to SceneGraph.
* Schedule or request another execution slice.

This prevents an accidental infinite loop from immediately freezing the interface.

## Choice Resumption

When the menu returns a selection:

1. Read the selected choice target.
2. Clear the pending-choice list.
3. Jump to the selected label.
4. Set status to `RUNNING`.
5. Resume VM execution.

## Error Handling

Catch or detect:

* Unknown operation
* Missing label
* Invalid instruction shape
* Invalid choice
* Missing entry point
* Instruction limit repeatedly exceeded

Display the error on the television and print diagnostics to the console.

## Website Tasks

Create API endpoints:

```text
GET /api/atomic-games/{slug}/manifest
GET /api/atomic-games/{slug}/program
```

The manifest route should only expose successfully compiled and published games.

For local development, permit the proof-of-concept game to be loaded regardless of public publication status through a
protected development route or token.

## Basil Tasks

* Add a simple IR writer capable of emitting the hardcoded fixture.
* Add a command resembling:

```text
basilc --target atomic --emit-ir examples/atomic/haunted-door.atomic
```

For this week, it may still emit a fixed or minimally derived program.

* Confirm that ordinary Basil compilation remains unaffected.

## Day 5 Exit Gate

* The Roku VM executes a bundled JSON program.
* Text, image, title, and choices are driven by instructions.
* Choosing an option causes a jump and resumes execution.
* Invalid instructions display a controlled error.
* PHP API routes return a manifest and compiled program.

---

# Day 6 — Connect Roku to AtomicFlix.com

## Objective

Replace the bundled Roku program with the program compiled on the website.

## Roku Network Tasks

Create a Task-based loader:

```text
components/games/
    AtomicGameLoaderTask.xml
    AtomicGameLoaderTask.brs
```

The loader should:

1. Request the manifest.
2. Validate:

    * Format
    * Format version
    * Runtime version
    * Game ID
    * Revision
3. Request the program URL.
4. Parse the JSON.
5. Return the program to the game scene.
6. Start the VM.

Add visible loading states:

```text
Loading game manifest...
Loading game program...
Starting game...
```

Add error states:

```text
Game was not found.
The game could not be loaded.
The program is invalid.
This game requires a newer runtime.
```

## Reload Feature

Add a development control:

```text
Reload Current Game
```

Reload should:

1. Stop the VM.
2. Clear choices.
3. Reset the game renderer.
4. Cancel or invalidate existing tasks.
5. Fetch the newest manifest.
6. Fetch the newest program.
7. Start from the entry point.

Append a cache-busting query during development:

```text
?revision=12&developmentReload=timestamp
```

## Website Tasks

Add a development game list page or API endpoint:

```text
GET /api/atomic-games/development
```

It can return the current user’s game and two seeded demonstrations.

Add a prominent editor field:

```text
Current Published Revision: 12
```

Add a button:

```text
Compile and Publish to Development
```

For the proof-of-concept, publishing means making the newest compiled revision available to the Roku development
endpoint.

## Basil Tasks

* Run all workspace tests.
* Add documentation explaining:

    * AtomicBASIC is currently compiled by PHP for the proof-of-concept.
    * Rust currently owns the future target and shared IR structures.
    * The PHP compiler will eventually be replaced by or delegate to Basil.
* Ensure Rust-generated JSON and PHP-generated JSON use identical field names.

## Day 6 Exit Gate

* Roku loads the manifest from AtomicFlix.com.
* Roku loads the compiled program from AtomicFlix.com.
* The remotely loaded game runs.
* A source change on the website produces a new revision.
* Reloading on Roku displays the changed behavior.
* No Roku rebuild is needed for a game-source change.

This is the most important milestone of the week.

---

# Day 7 — Create Demonstrations, Diagnostics, and Documentation

## Objective

Prove the pipeline with several tiny programs and leave the projects in a stable, understandable state.

## Demonstration 1: Haunted Door

Tests:

* Image
* Title and body text
* Multiple choices
* Branching
* Scene transitions
* End state

Example flow:

```text
Open the door → unsettling message → end
Ring the bell → different message → end
Run away → safe ending
```

## Demonstration 2: Atomic Quiz

Add the optional commands:

```text
SET
ADD
IF_EQ
```

Tests:

* Variables
* Numeric values
* Scoring
* Conditional branching

Example source:

```basic
:start
SET score 0
TITLE "Atomic Quiz"
TEXT "What planet is known as the Red Planet?"
CHOICE "Mars" correct
CHOICE "Venus" wrong
WAITCHOICE

:correct
ADD score 1
TEXT "Correct!"
GOTO result

:wrong
TEXT "Incorrect."
GOTO result

:result
IF_EQ score 1 winner
GOTO loser

:winner
TITLE "You Win"
TEXT "Your score is 1."
END

:loser
TITLE "Try Again"
TEXT "Your score is 0."
END
```

## Demonstration 3: Remote Test Lab

Tests:

* `WAITKEY`, if completed
* Remote-key reporting
* Runtime diagnostics
* Clean exit and restart

This can simply display:

```text
Press a remote button.
```

Then report:

```text
You pressed LEFT.
```

This is a technical demonstration, not a game.

## Roku Diagnostics

Add a development overlay or screen showing:

```text
Game ID
Game revision
Runtime version
VM status
Instruction pointer
Current source line
Pending choices
Instruction count
Last operation
Last remote key
Last error
```

Add development commands:

```text
Reload Game
Restart Game
Show Diagnostics
Hide Diagnostics
Exit Game
```

## Website Cleanup

* Seed the demonstration games.
* Make compile errors readable.
* Confirm that one user cannot accidentally edit another user’s game.
* Confirm that an uncompiled source revision is not served to Roku.
* Add a visible warning that this is a development proof-of-concept.
* Add a copyable Roku load URL or slug.

## Basil Cleanup

* Run:

```text
cargo fmt --all
cargo clippy --workspace
cargo test --workspace
```

* Confirm:

    * Normal Basil examples still work.
    * Atomic target stubs build.
    * IR serialization tests pass.
* Commit a short design note:

```text
docs/ATOMIC_BASIC.md
```

Include:

* Current proof-of-concept architecture
* Shared IR contract
* What PHP currently compiles
* What will eventually move into Rust
* Future VM and browser targets
* Commands for testing the Atomic stubs

## End-to-End Test

Perform the complete workflow:

1. Open Haunted Door on Roku.
2. Play through one path.
3. Open the AtomicFlix editor.
4. Change a title, description, and branch result.
5. Compile and publish.
6. Note the new revision.
7. Select Reload Game on Roku.
8. Confirm that the changed program appears.
9. Introduce a deliberate source error.
10. Confirm that the website refuses to publish it.
11. Introduce a deliberate IR/runtime error in development.
12. Confirm that Roku displays an error and returns safely.

## Day 7 Exit Gate

* At least two demonstration games run remotely.
* The editor-to-Roku refresh loop works consistently.
* Runtime diagnostics exist.
* Compiler and runtime errors are distinguishable.
* AtomicFlix.com still operates normally.
* The normal Roku media interface still operates normally.
* Basil builds and contains functional AtomicBASIC target stubs.
* All contracts and setup commands are documented.

---

# Project Responsibility Summary

## AtomicFlix.com

Owns during the proof-of-concept:

* User-accessible editor
* Game database records
* Temporary PHP AtomicBASIC compiler
* Compiled JSON IR
* Manifest generation
* Program API
* Development publishing
* Revision management

## Basil

Owns during the proof-of-concept:

* AtomicBASIC target designation
* Shared Rust manifest and IR types
* JSON serialization
* IR validation
* Fixture tests
* Future compiler integration point
* Confirmation that the existing Basil environment remains healthy

Basil does not need to be on the critical path for the first Roku demonstration.

## AtomicFlix Roku App

Owns:

* Game loader
* BrightScript VM
* SceneGraph renderer
* Remote-control input
* Menu suspension and VM resumption
* Game restart and reload
* Runtime diagnostics
* Safe error recovery

---

# Recommended Daily Commit Points

## AtomicFlix Repository

```text
Day 1: chore: restore AtomicFlix and Roku development workflow
Day 2: docs: define AtomicBASIC manifest and IR v1
Day 3: feat: add static AtomicBASIC Roku sandbox
Day 4: feat: add AtomicBASIC editor and PHP compiler
Day 5: feat: add BrightScript Atomic Game VM
Day 6: feat: load compiled Atomic games from website API
Day 7: feat: add AtomicBASIC demos and runtime diagnostics
```

## Basil Repository

```text
Day 1: chore: restore Basil workspace build
Day 2: feat: add Atomic game IR data structures
Day 3: feat: register AtomicBASIC compilation target
Day 4: test: add Atomic IR validation fixtures
Day 5: feat: add stub Atomic IR emitter
Day 6: docs: document PHP-to-Basil compiler migration path
Day 7: chore: format, lint, test, and document AtomicBASIC stubs
```

---

# Features That Must Not Delay the Week-One Proof

Defer these even when they are tempting:

* Asset-upload system
* Audio and video playback
* Browser preview
* Syntax highlighting
* Autocomplete
* Multiple games per user
* Public game directory
* Favorites and history
* Cloud save data
* Arbitrary HTTP calls
* User-defined functions
* Multidimensional arrays
* Dictionaries
* Structured exception handling
* Optimized bytecode
* Rust-to-Roku compiler integration
* WebAssembly runtime
* Production security review
* Roku publication or certification work

The proof-of-concept is not successful because the language is sophisticated. It is successful because the editing,
compilation, delivery, execution, input, branching, and reload pipeline works from end to end.

---

# Most Important Architectural Rule

The shared JSON IR is the temporary boundary between all three systems:

```text
AtomicBASIC source
        |
        v
PHP proof-of-concept compiler
        |
        v
AtomicFlix Game IR
       / \
      /   \
 Rust     BrightScript
tests       Roku VM
```

Later, the PHP compiler can be replaced:

```text
AtomicBASIC source
        |
        v
Basil compiler
        |
        v
AtomicFlix Game IR or bytecode
       / \
      /   \
 Browser  Roku
   VM      VM
```

Because the Roku VM depends on the shared IR rather than on the PHP implementation, replacing the compiler later should
not require rewriting the game runtime.

The next practical artifact is a **Day 1–2 Junie prompt** covering environment restoration, shared schemas, fixtures,
and the initial Roku sandbox without letting Junie overbuild the language.
