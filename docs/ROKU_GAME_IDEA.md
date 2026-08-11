## Original note:

have an idea. So we have our Roku app for AtomicFlix which is a scaled-down version of our AtomicFlix.com website with
just the media, presented in a minimal Netflix-style TV menu. Our website is similar but has more style, and things like
user management, comments, likes, reviews, admin and other things. RIght now, AtomicFlix on Roku is just for watching
full movies and short video series chapters, podcasts, ASMR Sleep videos, and educational lessons. Now I want to add
simple games, but I want to do it in an interesting, unusual way. We played around with making a simple game engine on
the website which was essentially a non-linear storyboard presentation where users can select which direction to go,
using the current slide as an imaginary game location, and the options could be things like "Go Left", or "Go upstairs"
or "Ring the Doorbell" which would determine the next slide. I decided that this ultimately would be too boring to
continue working on LOL But now I have a new idea that can include this "non-linear storyboard" type of game, but could
also be used to make all sorts of games, and be playable on Roku, and allow users to create and share their own games
with incredible ease. So my idea is to build a BASIC interpreter into the Roku app, using BrightScript to parse,
interpret and execute simple scripts. It would be a very targeted implementation of BASIC which could primarily do the
following things: 1. Display images and videos along with text in a small variety of pre-determined screen layouts such
as "Full screen", "Full screen with text", "Left Media with Right text", etc. 2. Display limited modal dialogs with
opaque or transparent backgrounds for building menus, notifications, alerts, etc. 3. Stop and wait for input from the
remote control 4. Respond to an asyncronous event from the remote control 5. Display a small number of interactive modal
menus such as the Javascript "alert", "confirm" and "prompt" dialogs, as well as simple prefabricated interactive menu
dialogs which accept an array of options as a parameter and returns the selected element if an option is selected 6.
Play audio while waiting for a response from any type of blocking input 7. Set timeout and interval functions like
Javascript 8. Call a REST API syncronously or asyncronously. 9. Support complex data types which can be automatically
converted to and from JSON in API calls 10. Have some support for lower level UI functions such as being able to
position a visual element on the screen, such as text or primitive graphics, with size, color, coordinates, and/or
opacity properties if possible 11. Support a full range of BASIC logic, string, arithmetic, and other built-in functions
to be able to intelligently execute somewhat complex algorithms for game play. For instance, it should be capable of
coding games like Tic Tac Toe, Checkers, Hangman, or even porting something like ELIZA. It should support
multi-dimensional arrays, user-defined functions, lists, dictonaries and other common data types, and reasonable error
handling. 12. Pre-fetch assets such as icons, backgrounds, or images via asset declaration commands at the start of the
programs in order to improve performance during program execution (if that's possible -- I imagine that just "touching"
an asset would speed up subsequent requests to fetch it, perhaps?) Obvously it would not need to support things like
file I/O, keyboard, mouse, printer, memory access, or other common features of BASIC that you would expect to find on a
typical PC based implementation. This would allow me to create the "non-linear storyboard" type of game, but also any
other sort of simple 2-D game without having to republish the App or even the Roku Feed data on the CDN. We could simply
load the game code using an API call to the AtomicFlix website, and reference any media assets from the website (or the
associated CDN storage) directly during code execution. Then we could create a page on the website where a user can edit
the code of their game and upload assets, and do minimal debugging. This would mean that whatever we write in
BrightScript would need to be mirrored in Javascript on the website to a degree. The web version doesn't need to be
production ready, just good enough so the user can verify that the game may work as expected on Roku. This could be a
Phase 2 feasture, however. The important thing is to have it working in Roku, as it would be simple enough to edit and
reload the game programs directly from Roku. That's the best part, being able to edit the game on the website and
immediatly reload the game program directly from Roku with no build step in between! The name of the game would be, by
default, the user's AtomicFlix username. For Phase I, only one game per user would be allowed, but in Phase II we could
allow a user to create and name multiple games. For Phase I, if a user wanted to have two different games they would
need to have 2 user accounts on the website. This lets us defer having to worry about coding those features for now. I
want to focus primarily on our BASIC platform. Finally, we would support a "Game" type in our Roku Feed object, which
would display a poster or thumbnail just like our current series, chapter, podcast and blitz types do now, and it would
directly load and run the game, but we would also have a "Load Game" option on the main AtomicFlix display in which a
user can use the alphanumeric text picker widget to type in the username of the game developer in order to load and run
the game, and then we will give them the ability to bookmark games in their main feed somehow, or just let them pick an
item from a history dropdown instead of entering a new game name. This would allow us to highlight featured games. This
would allow anyone to create a game for our Roku app. This could start a new beginner's ecosystem of Roku game creators
We could also tie this in nicely with our BasilBASIC and YoBASIC online learning goals and give students something
really impactful to experiment, create and share with minimal technical load What do you think?

## I think this may be one of our best AtomicFlix ideas

The original nonlinear storyboard was a **game**, but only one narrowly defined kind of game. This new idea turns that
abandoned experiment into a **general-purpose television game platform**.

What you are describing is essentially:

> **A remotely updatable, sandboxed game virtual machine inside AtomicFlix, programmed with a beginner-friendly BASIC
dialect and rendered through Roku SceneGraph.**

That is far more interesting than merely adding a few games. It gives AtomicFlix an unusual feature that Netflix,
YouTube, and most Roku media apps do not have:

* Users can make something on the website.
* Save it.
* Walk over to the television.
* Enter their username.
* Immediately play it.
* Share the username with someone else, who can immediately play it too.
* No Roku packaging, sideloading, app publishing, or certification cycle for each game.

That last point is the killer feature.

## It is technically feasible—with one major architectural adjustment

I would **not** have Roku download raw BASIC source, fully parse it, and execute it directly as the primary production
architecture.

Instead:

1. The user writes BASIC source on AtomicFlix.com.
2. The server parses, validates, and compiles it into a versioned **AtomicFlix Game IR or bytecode**.
3. Roku downloads that compact compiled representation.
4. A BrightScript virtual machine executes it.
5. The website preview runs the same representation through a JavaScript or WebAssembly VM.

The BASIC language remains completely real. The user still writes BASIC. The difference is that parsing and validation
happen on the server, where debugging, error reporting, security scanning, and future language upgrades are vastly
easier.

You could still eventually include a Roku-side parser for experimentation, but it should not be required for ordinary
game execution.

This also solves your “mirror everything in JavaScript” problem. You do not need two identical BASIC parsers and
compilers. You need:

* One canonical compiler.
* One portable instruction format.
* One Roku VM.
* One browser VM.

Your existing **Basil infrastructure** makes this especially attractive. This could become a restricted **Basil Game
Profile**, compiled by `basilcore` into an AtomicFlix-specific bytecode. The browser could eventually use `basil-wasm`,
while Roku gets a deliberately small BrightScript bytecode interpreter.

## The runtime should be event-driven, not truly blocking

Your BASIC language can *appear* to contain blocking statements:

```basic
choice = CHOOSE(["Ring the doorbell", "Go upstairs", "Run away"])
PRINT choice
```

But the BrightScript interpreter must not literally block the Roku SceneGraph render thread while waiting. Roku warns
that blocking the render thread causes production apps to terminate after ten seconds. Network and other potentially
blocking work belongs in Task threads. ([Roku][1])

Instead, the VM would execute like this:

1. Run instructions until reaching `CHOOSE`.
2. Create the menu.
3. Store the VM instruction pointer and current stack.
4. Mark the VM as `WAITING_FOR_MENU`.
5. Return control to Roku.
6. Roku receives a remote-control event.
7. The menu produces a result.
8. The VM resumes after `CHOOSE`.

The same mechanism handles:

* `WAITKEY`
* `PROMPT`
* `CONFIRM`
* Video completion
* Audio completion
* Timer events
* Asynchronous API responses
* Animations
* Scene transitions

Roku SceneGraph already has remote-control focus and event propagation through `onKeyEvent()`, so your runtime can
funnel remote input into the suspended VM. ([Roku][2])

This continuation-based design is probably the single most important decision in the entire project.

## A clean internal architecture

I would divide the Roku implementation into six layers.

### 1. Game Loader

Loads a game package by username, featured-game ID, bookmark, or history item.

The package might resemble:

```json
{
  "gameId": "erik",
  "revision": 42,
  "runtimeVersion": 1,
  "title": "Erik",
  "poster": "https://cdn.atomicflix.com/games/erik/poster.jpg",
  "program": "https://cdn.atomicflix.com/games/erik/game.afbc",
  "assets": [
    {
      "id": "front_door",
      "type": "image",
      "url": "https://cdn.atomicflix.com/games/erik/front-door.webp"
    }
  ]
}
```

The `revision` allows Roku to know immediately whether its cached copy is stale.

### 2. Atomic Game VM

This owns:

* Instruction pointer
* Operand stack
* Call stack
* Global variables
* Local variables
* Arrays and dictionaries
* Timer registrations
* Event handlers
* Current waiting state
* Error state
* Execution quota

The VM executes a controlled set of bytecode operations such as:

```text
PUSH_CONST
LOAD_GLOBAL
STORE_GLOBAL
ADD
COMPARE_EQ
JUMP
JUMP_IF_FALSE
CALL
RETURN
SHOW_LAYOUT
SHOW_IMAGE
SHOW_TEXT
OPEN_MENU
WAIT_EVENT
HTTP_REQUEST
PLAY_AUDIO
```

### 3. SceneGraph Renderer

The VM should never manipulate arbitrary SceneGraph objects directly.

Instead, it sends high-level commands to the renderer:

```text
SET_LAYOUT "media-left-text-right"
SET_BACKGROUND assetId
SET_TITLE "The Abandoned House"
SET_BODY "The porch light is flickering."
SHOW_CHOICES [...]
```

The renderer owns a small pool of reusable SceneGraph nodes. This prevents games from constructing thousands of nodes or
accidentally destroying the AtomicFlix interface.

Roku provides positioning, translation, scaling, visibility, and opacity through SceneGraph Group-derived nodes, so
primitive graphics and positioned elements are practical. ([Roku][3])

### 4. Input and Dialog Manager

This would implement the BASIC-facing functions:

```basic
ALERT("You found a key.")
answer = CONFIRM("Open the door?")
name = PROMPT("What is your name?")
choice = CHOOSE(options)
key = WAITKEY()
```

Roku has standard message, keyboard, pin-pad, progress, and customizable dialog components, including voice-capable
keyboard entry. ([Roku][4])

For game menus, however, you may prefer your own SceneGraph overlay so that games have a consistent AtomicFlix visual
identity.

### 5. Job Manager

The job manager handles:

* HTTP requests
* Timers
* Asset downloads
* Background JSON processing
* Possibly game save synchronization

Network calls run through Task nodes and report their result back to the VM as events. Timers can similarly use observed
SceneGraph fields to invoke callbacks. ([Roku][5])

The BASIC programmer could write synchronous-looking code:

```basic
weather = HTTP.GET("api/weather")
PRINT weather.temperature
```

Internally, `HTTP.GET` suspends the VM until a Task produces a result.

You could also support explicit asynchronous code later:

```basic
HTTP.GETASYNC("api/weather", WeatherLoaded)

FUNCTION WeatherLoaded(result)
    PRINT result.temperature
END FUNCTION
```

### 6. Asset Manager

The asset declaration idea is excellent:

```basic
ASSET IMAGE FOYER = "foyer.webp"
ASSET IMAGE DOOR = "door.webp"
ASSET AUDIO WIND = "wind.mp3"
ASSET VIDEO INTRO = "intro.mp4"
```

At startup, the server-provided manifest tells Roku what assets are needed. The runtime can preload critical images and
audio, report progress, and retain useful files in `cachefs:`. Roku specifically supports `cachefs:` as evictable
application caching storage; unlike `tmp:`, it does not count toward the app’s normal memory total, although it can be
evicted and disappears after reboot. ([Roku][6])

Remote Poster images already load asynchronously, but merely “touching” an image does not give you enough explicit
control. A real asset manager is preferable. Roku also recommends loading images near their intended display dimensions
to avoid texture-memory and performance problems. ([Roku][7])

## The language should be BASIC—but optimized for television games

I would avoid slavishly recreating GW-BASIC or Visual Basic. Make it recognizably BASIC while giving it first-class
television concepts.

For example:

```basic
GAME "The House on Briar Road"

ASSET IMAGE HOUSE = "house.webp"
ASSET IMAGE HALLWAY = "hallway.webp"
ASSET AUDIO WIND = "wind.mp3"

DIM inventory AS LIST
DIM player AS MAP

player.name = "Visitor"
player.health = 10

SCENE FrontDoor

    LAYOUT "fullscreen-text"
    SHOW IMAGE HOUSE
    PLAY AUDIO WIND LOOP

    TITLE "The House on Briar Road"
    TEXT "The front door is slightly open."

    choice = CHOOSE([
        "Ring the doorbell",
        "Open the door",
        "Run away"
    ])

    SELECT CASE choice
        CASE 0
            ALERT "Nobody answers."
            GOTO FrontDoor

        CASE 1
            GOTO Hallway

        CASE 2
            END GAME
    END SELECT

END SCENE


SCENE Hallway

    SHOW IMAGE HALLWAY
    TEXT "Something moves at the end of the hall."

    SETTIMEOUT Footsteps, 3000
    key = WAITKEY()

END SCENE


FUNCTION Footsteps()
    PLAY AUDIO "footsteps.mp3"
    ALERT "It is getting closer."
END FUNCTION
```

`SCENE` is not traditional BASIC, but it makes the language almost absurdly easy for story-game creators. Internally, it
can simply compile into a labeled function.

For board games:

```basic
DIM board(3, 3)

FUNCTION MakeMove(row, column, player)
    IF board(row, column) <> "" THEN
        RETURN FALSE
    END IF

    board(row, column) = player
    RETURN TRUE
END FUNCTION
```

For ELIZA-like programs, the language would need:

* String splitting
* Pattern matching
* Replacement
* Random selection
* Lists and dictionaries
* Functions
* Loops
* JSON data loading

All of those fit comfortably into the VM model. BrightScript itself already has arrays, associative arrays, and JSON
conversion primitives, which makes implementing corresponding VM values relatively natural. ([Roku][8])

## I would add two capabilities not mentioned in your list

### Controlled game-state persistence

Games will quickly need:

```basic
SAVE "chapter", 7
SAVE "inventory", inventory

chapter = LOAD("chapter", 1)
```

This should not be general-purpose file access. It should be a safe game-state service keyed by:

* AtomicFlix user
* Game ID
* Save slot
* Data key

Initially it could save locally. Eventually it could synchronize through AtomicFlix so someone can continue a game on
another Roku.

### Deterministic random numbers

Support:

```basic
RANDOMIZE
roll = RND(1, 6)
```

But also allow a known seed:

```basic
RANDOMIZE 12345
```

That makes bugs reproducible and allows the web preview and Roku runtime to produce identical results during testing.

## The security sandbox is essential

Because this is user-created code running inside your published app, the language needs strict limits from day one.

I would enforce:

* No `EVAL`.
* No access to BrightScript objects.
* No arbitrary SceneGraph node creation.
* No arbitrary file paths.
* No access to AtomicFlix authentication tokens.
* No unrestricted HTTP destinations in Phase I.
* Maximum instructions per execution slice.
* Maximum total variables and collection sizes.
* Maximum call-stack depth.
* Maximum timers and network requests.
* Maximum downloaded response size.
* Maximum SceneGraph elements.
* Maximum asset count and storage size.
* Maximum execution time without yielding.
* Automatic cancellation when the user exits the game.
* A runtime error screen that returns safely to AtomicFlix.

The instruction limit is especially important:

```text
Run up to 5,000 VM instructions.
If the program has not yielded:
    schedule another execution slice
    return control to SceneGraph
```

That lets computational code run without freezing the television.

For REST calls, Phase I should probably allow only:

```basic
API.GET("/games/my-data")
API.POST("/games/my-score", score)
```

Those calls would go through AtomicFlix’s server. Do not initially allow:

```basic
HTTP.GET("https://anything-on-the-internet.example")
```

Otherwise a game author could use thousands of Roku installations as HTTP clients, exfiltrate information, attack
endpoints, display unreviewed remote content, or create certification problems.

## What kinds of games would work especially well?

This platform would be excellent for:

* Choose-your-own-adventure stories
* Visual novels
* Trivia games
* Educational quizzes
* Hangman
* Tic-tac-toe
* Checkers
* Card games
* Puzzle games
* Turn-based RPGs
* Murder mysteries
* Escape-room games
* Game-show simulations
* Interactive podcasts
* Personality tests
* ELIZA and chatbot-style characters
* Text adventures with rich media
* Party games where people take turns with the remote
* Children’s educational activities
* Interactive versions of Atomic Zone stories

It would be less suitable initially for:

* Platform games
* Physics games
* Fast arcade shooters
* Smooth 60-frame-per-second animation
* Games requiring simultaneous buttons
* Anything expecting a conventional game controller

You could eventually make simple sprite animation possible, but I would market Phase I as **interactive television games
**, not as a general arcade engine.

## Phase I should be aggressively constrained

I would ship the first runtime with approximately this scope:

### Presentation

* Six predefined screen layouts
* Image display
* Fullscreen video
* Background audio
* Text title and body
* Buttons and selectable menus
* Alert, confirm, prompt, and choose
* Rectangles, labels, and images at coordinates
* Basic fades and transitions

### Language

* Numbers, strings, Booleans, and null
* Lists and dictionaries
* One- and multidimensional arrays
* Variables and constants
* `IF`, `SELECT CASE`, `FOR`, `WHILE`
* Functions and return values
* Scenes or labels
* `TRY/CATCH`
* String and math functions
* JSON conversion
* Random numbers

### Events

* Remote key input
* Menu selection
* Timer
* Video finished
* Audio finished
* API response
* App pause or exit

### Platform

* One game per username
* Draft versus published status
* Featured games
* Load by username
* Recently played
* Favorites
* Reload newest revision
* Local game progress
* AtomicFlix-hosted assets only

That is already enough to build hundreds of genuinely different games.

## The best first three demonstration games

I would deliberately build three reference games that stress completely different parts of the runtime.

### 1. The Abandoned House

A media-heavy nonlinear storyboard.

Tests:

* Layouts
* Images
* Video
* Audio
* Menus
* Scenes
* Save state
* Asset preloading

### 2. Atomic Tic-Tac-Toe

A logic-heavy board game.

Tests:

* Multidimensional arrays
* Loops
* Functions
* Coordinate-based drawing
* Remote focus
* Win detection
* Replay

### 3. Doctor Atomic

An ELIZA-style conversational character.

Tests:

* Prompt dialog
* Strings
* Pattern matching
* Lists
* Dictionaries
* Random responses
* JSON-loaded dialog data

If the same VM can support those three without game-specific BrightScript code, you have proven the architecture.

## The web editor can initially be extremely simple

Phase I does not need a full IDE. It could contain:

* Source-code textarea
* Save button
* Compile button
* Compiler errors with line numbers
* Publish/unpublish toggle
* Asset upload area
* Poster and title fields
* Generated game revision number
* “Reload on Roku” instructions
* A JSON dump of the compiled game for debugging

The user workflow becomes:

1. Edit game.
2. Press Save.
3. Server compiles it.
4. Pick **Reload Game** on Roku.
5. Roku sees revision 43 instead of 42.
6. Roku downloads the new bytecode and assets.
7. Game restarts.

That is close to the immediacy of editing a webpage and refreshing the browser, except the result appears on a
television.

## One policy issue deserves early investigation

Roku’s current certification documentation explicitly says that changing an app’s implementation code requires
re-certification, while changing streamed content does not. Your interpreted game programs sit somewhere between
“content” and “implementation code.” ([Roku][9])

Because the downloaded program cannot access Roku directly and can only issue predefined AtomicFlix presentation
commands, you have a strong argument that it is structured interactive content rather than downloaded BrightScript
implementation code. Nevertheless, I would ask Roku Developer Support for a written interpretation before opening
unrestricted public game publishing.

Describe it as:

> “A declarative interactive-content format interpreted by a fixed, certified SceneGraph runtime.”

Do **not** lead with:

> “Our app downloads and executes user-written programs.”

Both descriptions may refer to the same architecture, but the first more accurately emphasizes the sandbox boundary.

You also need games to work acceptably across all currently supported Roku models, not merely newer devices, because
Roku’s certification policy requires apps and their content to remain usable across devices receiving the current Roku
OS. ([Roku][9])

## My strongest recommendation

Make this part of Basil rather than inventing a completely separate BASIC.

Something like:

* **Basil** — the full language.
* **Basil TV Profile** — the restricted language specification.
* **Atomic Game Runtime** — the Roku implementation.
* **AtomicBASIC** — the user-facing creative environment.
* `.bas` — source.
* `.afgame` — manifest.
* `.afbc` — compiled AtomicFlix bytecode.

This creates a remarkable educational progression:

1. A student learns basic programming in YoBASIC.
2. The same core concepts work in Basil.
3. They use AtomicBASIC to make a television game.
4. They publish it on AtomicFlix.
5. Friends and family type their username into a Roku.
6. Their creation appears instantly on the television.

That is not merely a game feature. It is a concrete reason for beginners to learn programming: **“Write code, and
tonight your game is playable on an actual TV.”**

The original storyboard project was too boring because you were building individual branches and slides. This idea turns
that discarded prototype into the first sample application for a programmable, community-created interactive television
platform. That is much bigger—and unusually well aligned with AtomicFlix, Basil, YoBASIC, and your educational goals.

[1]: https://developer.roku.com/dev/docs/threads?utm_source=chatgpt.com "Threads"

[2]: https://developer.roku.com/docs/developer-program/core-concepts/scenegraph-xml/remote-control-events.md?utm_source=chatgpt.com "Remote control events"

[3]: https://developer.roku.com/dev/docs/group?utm_source=chatgpt.com "Group"

[4]: https://developer.roku.com/docs/references/scenegraph/standard-dialog-framework-nodes/standard-dialog.md?utm_source=chatgpt.com "StandardDialog"

[5]: https://developer.roku.com/dev/docs/handling-application-events?utm_source=chatgpt.com "Handling application events"

[6]: https://developer.roku.com/docs/developer-program/getting-started/architecture/file-system.md?utm_source=chatgpt.com "File system"

[7]: https://developer.roku.com/dev/docs/poster?utm_source=chatgpt.com "Poster"

[8]: https://developer.roku.com/dev/docs/global-utility-functions?utm_source=chatgpt.com "Global utility functions"

[9]: https://developer.roku.com/dev/docs/certification?utm_source=chatgpt.com "Certification criteria"
