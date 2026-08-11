The proof-of-concept should validate the **entire creative pipeline**, not attempt to build a miniature production
platform prematurely.

# AtomicFlix BASIC Games

## High-Level Proof-of-Concept Roadmap

## 1. Proof-of-Concept Objective

Create a side-loaded development version of the AtomicFlix Roku app that can:

1. Display a game inside the existing AtomicFlix interface.
2. Retrieve a game manifest and program from AtomicFlix.com.
3. Execute a small BASIC-derived instruction set.
4. Display text, images, primitive graphics, and simple menus.
5. Pause execution while waiting for remote-control input.
6. Resume the program after a selection, key press, timer, or API response.
7. Edit the game on AtomicFlix.com and reload the new revision on Roku without rebuilding or resideloading the app.
8. Run several tiny demonstration games that exercise different parts of the system.

The proof-of-concept is successful when we can change a BASIC program on the website, save it, select **Reload Game** on
the Roku, and immediately see the changed behavior.

That end-to-end refresh loop is the main product we are proving.

---

# 2. Proposed Proof-of-Concept Architecture

```text
AtomicFlix Web Editor
        |
        | BASIC source
        v
AtomicFlix Game Compiler
        |
        | validated AFG-IR JSON
        v
AtomicFlix Game API / CDN
        |
        | manifest, program, assets
        v
Roku Game Loader
        |
        v
BrightScript Game VM
        |
        +----> SceneGraph Renderer
        +----> Remote Input Manager
        +----> Audio/Video Manager
        +----> Timer Manager
        +----> API Task Manager
```

For the proof-of-concept, I recommend that **Roku execute a compiled JSON instruction format rather than parse the
complete BASIC source itself**.

The BrightScript component remains a genuine interpreter or virtual machine. It simply interprets AtomicFlix Game
Instructions rather than performing all lexical analysis, parsing, validation, and compilation on the Roku.

This gives us one canonical compilation pipeline:

```text
BASIC source → AtomicFlix Game IR
```

Later:

```text
AtomicFlix Game IR → Roku BrightScript VM
AtomicFlix Game IR → Browser JavaScript VM
```

That avoids maintaining two separate BASIC parsers.

We can include the original BASIC source in development API responses for diagnostics, but the Roku runtime should
initially execute the compiled instructions.

---

# 3. Proof-of-Concept Boundaries

## Included

The proof-of-concept will support:

* One development game or a small set of hardcoded test games
* A game manifest
* Remote program loading
* Program revision numbers
* A small virtual machine
* Variables
* Numbers, strings, Booleans, lists, and maps
* Basic arithmetic and comparisons
* Conditional branching
* Simple loops
* Labels or scenes
* Text display
* Image display
* Primitive rectangles
* A few predefined layouts
* Remote-control input
* Selectable menus
* Audio playback
* Timers
* One controlled AtomicFlix API request
* Runtime errors
* Program reloading
* Several demonstration games

## Explicitly Deferred

The proof-of-concept will not attempt to provide:

* Public game publishing
* Multiple games per user
* Production user permissions
* Community moderation
* A polished online IDE
* Browser game emulation
* Arbitrary external HTTP requests
* Cloud game saves
* Game ratings or reviews
* Sophisticated animation
* High-performance arcade games
* Complete Basil compatibility
* Complete BASIC syntax
* Production-level backward compatibility
* Roku Store certification
* Final content-security policy
* Monetization
* Multiplayer support

Anything not needed to prove the complete pipeline should wait.

---

# 4. Phase Zero: Define the Contracts

Before constructing the VM, define the three contracts that everything else will use.

## 4.1 Game Manifest Contract

A proof-of-concept manifest might look like:

```json
{
  "format": "atomicflix-game",
  "formatVersion": 1,
  "runtimeVersion": 1,
  "gameId": "erik",
  "revision": 7,
  "title": "Erik's Test Game",
  "programUrl": "https://atomicflix.com/api/games/erik/program",
  "posterUrl": "https://cdn.atomicflix.com/games/erik/poster.jpg",
  "assets": [
    {
      "id": "front-door",
      "type": "image",
      "url": "https://cdn.atomicflix.com/games/erik/front-door.jpg"
    },
    {
      "id": "wind",
      "type": "audio",
      "url": "https://cdn.atomicflix.com/games/erik/wind.mp3"
    }
  ]
}
```

## 4.2 Instruction Contract

The initial AtomicFlix Game IR—temporarily called **AFG-IR**—should be readable JSON:

```json
{
  "entryPoint": "start",
  "variables": {
    "visits": 0
  },
  "instructions": [
    {
      "op": "LABEL",
      "name": "start"
    },
    {
      "op": "SET_LAYOUT",
      "layout": "fullscreen-text"
    },
    {
      "op": "SHOW_IMAGE",
      "asset": "front-door"
    },
    {
      "op": "SHOW_TEXT",
      "title": "The Old House",
      "body": "The front door is slightly open."
    },
    {
      "op": "CHOOSE",
      "target": "choice",
      "options": [
        "Open the door",
        "Ring the bell",
        "Run away"
      ]
    },
    {
      "op": "BRANCH",
      "value": {
        "variable": "choice"
      },
      "cases": {
        "0": "openDoor",
        "1": "ringBell",
        "2": "finish"
      }
    }
  ]
}
```

Readable JSON is less compact than a binary bytecode, but it is dramatically easier to inspect during the
proof-of-concept.

A compact bytecode can come later.

## 4.3 Renderer Contract

The VM should issue high-level renderer commands. It should not permit game programs to manipulate arbitrary SceneGraph
nodes.

Examples:

```text
ClearScreen
SetLayout
ShowImage
ShowText
ShowRectangle
ShowMenu
PlayAudio
PlayVideo
SetElementPosition
SetElementOpacity
```

SceneGraph’s basic renderable elements include `Rectangle`, `Label`, and `Poster`, which can be composed into custom UI
components. This is enough to construct the initial layouts, menus, board-game cells, text overlays, and
images. ([Roku][1])

## Phase-Zero Exit Gate

We should have:

* A written manifest schema
* A written AFG-IR schema
* A list of initial VM operations
* A list of predefined layouts
* One hand-written test program in JSON

No compiler is required yet.

---

# 5. Phase One: Create the Roku Game Sandbox

Add a temporary development entry point to the AtomicFlix Roku app.

This could be:

```text
AtomicFlix Development
    ├── Normal AtomicFlix
    └── BASIC Game Sandbox
```

Alternatively, the sandbox can be opened using an undocumented remote-control sequence so it does not interfere with the
normal application.

## Initial Sandbox Behavior

The sandbox should:

1. Open a new `GameRunnerScene` or equivalent custom component.
2. Show a hardcoded background.
3. Show a title and body text.
4. Display a three-option menu.
5. Receive remote-control movement.
6. Report the selected option.
7. Return safely to AtomicFlix when Back is pressed.

Roku SceneGraph assigns remote focus to one node at a time and routes key events through the focus chain. Custom
components can handle keys through `onKeyEvent()`, which fits the proposed input-manager architecture. ([Roku][2])

## Suggested Roku Components

```text
components/
    GameRunnerScene.xml
    GameRunnerScene.brs

    GameCanvas.xml
    GameCanvas.brs

    GameMenu.xml
    GameMenu.brs

    GameDialog.xml
    GameDialog.brs
```

## Phase-One Exit Gate

On a side-loaded development Roku:

* The game sandbox opens.
* An image and text appear.
* Up and Down change the focused option.
* OK returns the selected option.
* Back closes the sandbox.
* Normal AtomicFlix functionality remains intact.

Roku’s current development workflow supports enabling developer mode and uploading a side-loaded package through the
device’s Development Application Installer. Only one development app is side-loaded at a time, so this should initially
be a development build of the existing AtomicFlix app rather than a separate simultaneous test app. ([Roku][3])

---

# 6. Phase Two: Build the Smallest Possible VM

Implement a BrightScript interpreter for a hand-written AFG-IR program bundled with the Roku app.

Do not involve the website or networking yet.

## Initial VM State

```text
program
instructionPointer
variables
labels
callStack
status
waitState
lastError
```

Possible statuses:

```text
READY
RUNNING
WAITING_FOR_INPUT
WAITING_FOR_TIMER
WAITING_FOR_API
WAITING_FOR_MEDIA
FINISHED
ERROR
```

## Initial Instruction Set

The first interpreter only needs:

```text
LABEL
SET
ADD
COMPARE
JUMP
JUMP_IF
SET_LAYOUT
SHOW_TEXT
SHOW_IMAGE
SHOW_RECTANGLE
CHOOSE
WAIT_KEY
PLAY_AUDIO
STOP_AUDIO
END
```

## Execution Slicing

The VM should execute a limited number of instructions and then return control to SceneGraph:

```text
executeSlice(maxInstructions)
```

Conceptually:

```basic
while instructionsExecuted < maxInstructions
    executeCurrentInstruction()

    if m.status <> "RUNNING"
        exit while
    end if

    m.instructionPointer++
end while
```

This avoids letting a game’s loop monopolize the interface.

## Suspension and Resumption

When the interpreter reaches:

```text
CHOOSE
```

it should:

1. Create or populate the menu.
2. Store the destination variable.
3. Change its status to `WAITING_FOR_INPUT`.
4. End the execution slice.
5. Return control to SceneGraph.

When a menu selection arrives:

1. Store the selected value.
2. Change the VM status to `RUNNING`.
3. Schedule another execution slice.

This establishes the continuation model that timers, video events, and API calls will later use.

## Phase-Two Exit Gate

A bundled JSON program can:

* Set variables
* Perform arithmetic
* Branch
* Show two different screens
* Wait for a menu selection
* Resume at the correct instruction
* Finish cleanly
* Detect an invalid opcode and show a useful error

---

# 7. Phase Three: Create the Game Presentation Layer

Replace hardcoded sandbox visuals with renderer commands driven entirely by the VM.

## Predefined Layouts

Start with four layouts:

### Full Screen

```text
[                    MEDIA                    ]
```

### Full Screen with Text Overlay

```text
[                    MEDIA                    ]
[             TRANSPARENT TEXT BOX            ]
```

### Media Left, Text Right

```text
[        MEDIA        ][         TEXT         ]
```

### Game Board

```text
[                  TITLE                     ]
[                                              ]
[               GAME CANVAS                   ]
[                                              ]
[               STATUS TEXT                   ]
```

## Primitive Elements

Support only:

* Text
* Image
* Rectangle

Each can have a developer-assigned ID:

```basic
RECT "cell1", 100, 100, 150, 150, "#ffffff", 1.0
TEXTAT "mark1", "X", 150, 130, 64, "#ff0000"
IMAGEAT "player", PLAYER_ICON, 400, 300, 80, 80
```

The VM translates those into safe renderer requests.

## Menus and Dialogs

The proof-of-concept needs:

* `ALERT`
* `CONFIRM`
* `CHOOSE`
* `WAITKEY`

`PROMPT` can be added only if the existing AtomicFlix alphanumeric picker can be reused cheaply. It is not required to
prove the core game loop.

## Phase-Three Exit Gate

A bundled game can:

* Change layouts
* Replace images and text
* Display transparent overlays
* Position primitive elements
* Display a selectable menu
* Wait for an arbitrary remote key
* Clear and redraw the game canvas

---

# 8. Phase Four: Add Timers, Audio, and Media Events

Once the continuation model is working, timers and media become additional event sources rather than special
architectural problems.

## Initial Timer Operations

```text
WAIT
SET_TIMEOUT
CANCEL_TIMEOUT
```

For example:

```basic
SHOW TEXT "Something is approaching..."
WAIT 2000
PLAY AUDIO FOOTSTEPS
SHOW TEXT "It is directly behind you."
```

Internally, `WAIT` suspends the VM and uses a SceneGraph `Timer` node. Roku field observers and Timer events can invoke
handlers when a timer fires. ([Roku][4])

## Initial Audio Operations

```text
PLAY_AUDIO
STOP_AUDIO
```

Support looping as an optional property:

```basic
PLAY AUDIO WIND LOOP
```

## Initial Video Operations

```text
PLAY_VIDEO
WAIT_VIDEO
STOP_VIDEO
```

Video does not initially need to coexist with complex game graphics. A video instruction can temporarily take over the
screen and return control to the game when playback finishes or the user exits.

## Phase-Four Exit Gate

A bundled program can:

* Display an image
* Play looping background audio
* Wait for remote input while audio continues
* Stop the audio
* Set a timer
* Resume after the timer
* Play a short video
* Resume after video completion

---

# 9. Phase Five: Load Remote Game Packages

Replace the bundled program with a remotely loaded manifest and AFG-IR file.

## Temporary Development Endpoint

```text
GET /api/roku-games/poc/manifest
GET /api/roku-games/poc/program
```

No user-facing game management is needed yet.

The manifest should contain:

* Game ID
* Game title
* Revision
* Required runtime version
* Program URL
* Asset declarations
* Optional poster URL

## Roku Loading Flow

```text
Open Game Sandbox
        |
Download Manifest
        |
Validate Format and Runtime Version
        |
Download Program
        |
Parse JSON
        |
Register Assets
        |
Start VM
```

BrightScript’s `ParseJson()` converts JSON objects into corresponding strings, numbers, arrays, and associative arrays,
which makes readable JSON suitable for the proof-of-concept instruction format. `FormatJson()` can assist with debugging
VM state and API payloads. ([Roku][5])

Network work should be isolated from the renderer and report results back through observed fields or the appropriate
asynchronous task mechanism.

## Error States

The loader should distinguish:

```text
Manifest unavailable
Program unavailable
Invalid JSON
Unsupported manifest version
Unsupported runtime version
Missing entry point
Missing asset
Program validation failure
```

## Phase-Five Exit Gate

The Roku loads and executes a game that is not packaged inside the application.

Changing the remote JSON program changes Roku behavior without resideloading the Roku app.

This proves the fundamental remote-content model.

---

# 10. Phase Six: Build the Minimal BASIC Compiler

Once remote IR execution works, introduce actual BASIC source.

The compiler can initially be implemented in PHP inside AtomicFlix.com. It does not need to be integrated into Basil
immediately, although its syntax and internal concepts should be chosen with Basil compatibility in mind.

## Tiny Proof-of-Concept Language

An initial source file might be:

```basic
GAME "The Old House"

ASSET IMAGE HOUSE = "front-door.jpg"
ASSET AUDIO WIND = "wind.mp3"

SCENE Start

    LAYOUT "fullscreen-text"
    SHOW IMAGE HOUSE
    PLAY AUDIO WIND LOOP

    TITLE "The Old House"
    TEXT "The front door is slightly open."

    choice = CHOOSE(
        "Open the door",
        "Ring the bell",
        "Run away"
    )

    IF choice = 1 THEN GOTO OpenDoor
    IF choice = 2 THEN GOTO RingBell
    GOTO Finish

END SCENE


SCENE OpenDoor

    TEXT "The door opens by itself."
    WAIT 1500
    ALERT "Something inside knows your name."
    GOTO Finish

END SCENE


SCENE RingBell

    TEXT "You hear the bell ringing somewhere behind you."
    WAITKEY
    GOTO Finish

END SCENE


SCENE Finish

    STOP AUDIO
    TEXT "The End"
    END GAME

END SCENE
```

## Minimal Compiler Responsibilities

The compiler should:

1. Normalize line endings.
2. Tokenize source lines.
3. Recognize commands.
4. Resolve labels and scenes.
5. Validate asset references.
6. Validate argument counts.
7. Convert expressions into simple expression trees.
8. Produce AFG-IR JSON.
9. Return line-numbered errors.
10. Increment the game revision after successful compilation.

## Minimal Syntax

Support only what the demonstrations require:

```text
GAME
ASSET
SCENE / END SCENE
DIM
assignment
IF / THEN
GOTO
SHOW
TITLE
TEXT
LAYOUT
ALERT
CONFIRM
CHOOSE
WAITKEY
WAIT
PLAY AUDIO
STOP AUDIO
RECT
TEXTAT
IMAGEAT
END GAME
```

Loops, functions, maps, multidimensional arrays, structured error handling, and asynchronous callbacks belong in the
production build-out plan unless a demonstration specifically requires one of them.

## Phase-Six Exit Gate

A BASIC source file is:

* Saved on AtomicFlix.com
* Compiled into AFG-IR
* Exposed through the game API
* Downloaded by Roku
* Executed by the BrightScript VM
* Reported with useful source-line errors when invalid

---

# 11. Phase Seven: Build the Minimal Website Workbench

The website component should be deliberately crude but usable.

## Development Page

```text
/admin/roku-game-poc
```

The page contains:

* Game title
* BASIC source textarea
* Save and Compile button
* Compiler output
* Current revision number
* Asset upload controls
* Current asset list
* Publish-to-development toggle
* Manifest preview
* Compiled AFG-IR preview
* Reset to sample program button

A full code editor is unnecessary. A textarea with a monospace font is sufficient.

## Asset Storage

For the proof-of-concept, assets can be stored under a single development namespace:

```text
games/poc/images/
games/poc/audio/
games/poc/video/
```

The compiler resolves:

```basic
ASSET IMAGE HOUSE = "front-door.jpg"
```

into the CDN URL included in the manifest.

## Phase-Seven Exit Gate

The developer can:

1. Edit BASIC in a browser.
2. Upload an image.
3. Compile the program.
4. See source errors.
5. Save a new revision.
6. Reload the game on Roku.
7. See the new source and asset immediately.

At this point, the central product concept has been proven.

---

# 12. Phase Eight: Add the Roku Development Game Browser

Replace the single hardcoded proof-of-concept URL with a small development game list.

## Development Game Menu

```text
BASIC Game Sandbox

    Haunted Door
    Atomic Grid
    The Oracle
    Load by Username
    Reload Current Game
    Clear Game Cache
    Show Runtime Diagnostics
```

The first three can initially be server-defined games.

## Reload Behavior

When **Reload Current Game** is selected:

1. Stop timers and media.
2. Cancel active network jobs.
3. Destroy or reset temporary game nodes.
4. Fetch the newest manifest.
5. Compare the revision.
6. Fetch the newest program.
7. Reset the VM.
8. Restart at the entry point.

The proof-of-concept should support a forced reload even when the revision has not changed, because caching errors will
otherwise complicate development.

## Diagnostics Screen

Show:

```text
Game ID
Game revision
Manifest version
Runtime version
Instruction pointer
VM status
Instruction count
Current scene
Last event
Last error
Loaded assets
Pending timers
Pending API requests
```

A diagnostics display will save enormous effort while debugging the interpreter on a television.

## Phase-Eight Exit Gate

The Roku development build can:

* List the demonstration games
* Load each one remotely
* Reload the current game
* Clear cached game data
* Show VM diagnostics
* Return to the normal AtomicFlix menu

---

# 13. The Three Proof-of-Concept Demonstrations

These are not intended to be complete games. Each one exists to prove a different technical capability.

## Demo One: Haunted Door

A tiny nonlinear visual story.

### Features Tested

* Game manifest
* Asset declarations
* Image loading
* Text overlays
* Background audio
* Choice menu
* Conditional branching
* Timers
* Scene changes
* Program completion

### Experience

The player sees a house and chooses:

```text
Open the door
Ring the bell
Walk away
```

Each option produces one or two different screens before ending.

This demonstrates the original nonlinear-storyboard concept.

---

## Demo Two: Atomic Grid

A minimal interactive board.

### Features Tested

* Remote directional input
* Cursor movement
* Positioned rectangles
* Positioned text
* Variables
* Basic arrays or a temporary list representation
* Arithmetic
* Conditional logic
* Redrawing
* Restarting

### Experience

The player moves a cursor around a three-by-three grid and presses OK to place an `X`.

It does not initially need:

* A second player
* A computer opponent
* Complete win detection
* A polished interface

Its purpose is to prove that the engine can implement something more interactive than a storyboard.

A slightly later revision can add alternating `X` and `O` marks and simple win detection.

---

## Demo Three: The Oracle

A tiny API-driven interactive character.

### Features Tested

* Text entry or menu-based questions
* AtomicFlix API request
* JSON request and response values
* Suspended VM execution
* API completion event
* String handling
* Timers
* Error handling

### Experience

The player selects a question:

```text
Will I become rich?
Is something behind me?
Should I open the door?
```

The game calls an AtomicFlix test endpoint:

```text
POST /api/roku-games/poc/oracle
```

The endpoint returns:

```json
{
  "answer": "Yes, but not in the way you expect.",
  "delay": 1500
}
```

The VM waits, displays the answer, and permits another question.

This proves that a game can communicate with AtomicFlix without exposing arbitrary network access.

---

# 14. Proof-of-Concept Definition of Done

The proof-of-concept is complete when all of the following are true:

## Roku Application

* AtomicFlix can open a game sandbox.
* A remotely hosted game manifest can be loaded.
* A remotely hosted program can be loaded.
* JSON instructions are interpreted by BrightScript.
* VM execution can suspend and resume.
* Remote input works.
* Images and text can be displayed.
* Primitive graphics can be positioned.
* Audio can continue during input.
* Timers can resume execution.
* An AtomicFlix API can be called.
* Runtime errors return safely to the game menu.
* The normal AtomicFlix app still works.

## Website

* BASIC source can be edited.
* Source can be compiled.
* Compilation errors include line numbers.
* Assets can be uploaded.
* A manifest is generated.
* A program revision is published.
* The compiled AFG-IR can be inspected.

## End-to-End Pipeline

* A source change is made on the website.
* The source is compiled.
* The Roku game is reloaded.
* The new behavior appears without rebuilding or resideloading AtomicFlix.

## Demonstrations

* Haunted Door runs.
* Atomic Grid runs.
* The Oracle runs.

---

# 15. Recommended Implementation Order

The order matters because every step should leave us with something visible and testable.

```text
1. Game sandbox with hardcoded UI
2. Remote-control menu
3. Hand-written bundled AFG-IR
4. Small BrightScript VM
5. VM suspension and resumption
6. VM-controlled rendering
7. Timers and audio
8. Remote manifest and program loading
9. PHP BASIC compiler
10. Website source editor
11. Asset upload and manifest generation
12. Roku reload workflow
13. Haunted Door
14. Atomic Grid
15. The Oracle
16. Diagnostics and proof-of-concept cleanup
```

The crucial discipline is not to start the website editor, elaborate BASIC syntax, user publishing system, or browser
preview until the Roku VM can already execute a hand-written remote program.

---

# 16. Suggested Proof-of-Concept Project Structure

## Roku

```text
components/
    games/
        GameRunnerScene.xml
        GameRunnerScene.brs

        GameCanvas.xml
        GameCanvas.brs

        GameMenu.xml
        GameMenu.brs

        GameDialog.xml
        GameDialog.brs

        GameNetworkTask.xml
        GameNetworkTask.brs

source/
    games/
        GameLoader.brs
        GameVM.brs
        GameRenderer.brs
        GameInputManager.brs
        GameAssetManager.brs
        GameDiagnostics.brs
```

## AtomicFlix Website

```text
app/
    Http/
        Controllers/
            RokuGamePocController.php
            Api/
                RokuGameApiController.php

    Services/
        Games/
            GameCompiler.php
            GameManifestBuilder.php
            GameAssetService.php

    GameLanguage/
        Lexer.php
        Parser.php
        Compiler.php
        CompilationError.php

resources/
    views/
        admin/
            roku-game-poc.blade.php

storage/
    app/
        games/
            poc/
                source/
                compiled/
                assets/
```

The compiler classes may later move into a separate Basil-related package once the language design stabilizes.

---

# 17. Decisions We Can Safely Defer

The proof-of-concept should produce evidence that helps answer these later questions:

* Should Roku execute JSON IR or compact bytecode?
* Should the canonical compiler live in PHP or Rust/Basil?
* How closely should AtomicBASIC match Basil?
* Should games use `SCENE`, line labels, functions, or all three?
* What is the correct per-game memory limit?
* How much arbitrary positioning should creators receive?
* Should external API access ever be allowed?
* How should games save progress?
* How should games be reviewed before publication?
* How should browser preview reproduce Roku focus behavior?
* How should game packages be versioned permanently?
* How should games appear in the existing Roku feed?
* What restrictions will Roku require for interpreted interactive content?

Those are production-MVP questions. We do not need to solve them to prove that the system works.

---

# 18. The First Concrete Milestone

The first implementation milestone should be extremely small:

> Add a hidden **Game Sandbox** option to the side-loaded AtomicFlix app. It opens a custom SceneGraph screen containing
> an image, a text description, and three selectable choices. Selecting a choice changes the image and text without
> leaving the screen.

That first milestone does not involve BASIC, networking, APIs, assets, compilers, or the website.

It proves the basic game presentation and input shell.

The second milestone replaces the hardcoded behavior with a bundled JSON instruction program.

From that moment forward, the application is no longer merely displaying a game mock-up—it is executing one.

After this proof-of-concept, the natural next document will be a component-by-component production MVP plan covering the
language/compiler, Roku VM, renderer, game API, web workbench, asset pipeline, security sandbox, publishing workflow,
and testing strategy.

[1]: https://developer.roku.com/dev/docs/creating-custom-components?utm_source=chatgpt.com "Creating custom components"

[2]: https://developer.roku.com/dev/docs/remote-control-events?utm_source=chatgpt.com "Remote control events"

[3]: https://developer.roku.com/dev/docs/developer-setup?utm_source=chatgpt.com "Activating developer mode"

[4]: https://developer.roku.com/dev/docs/handling-application-events?utm_source=chatgpt.com "Handling application events"

[5]: https://developer.roku.com/dev/docs/global-utility-functions?utm_source=chatgpt.com "Global utility functions"
