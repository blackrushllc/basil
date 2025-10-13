# Junie Prompt: Scaffold the “VS Code Basil” Extension (LSP + DAP)

Scaffold a minimal VS Code extension that registers the Basil language, provides a Monarch grammar, and launches both `basil-lsp` (Language Server) and `basil-dap` (Debug Adapter).

Task: Create a VS Code extension that:

1. Registers the Basil language (`.basil` files) with syntax highlighting (Monarch grammar).
2. Starts the `basil-lsp` language server over stdio for editor features.
3. Registers a “Basil” debugger type and launches `basil-dap` for debugging.
4. Provides a small README and sample workspace configs to test quickly.
5. Works on Windows/macOS/Linux and lets users configure paths to `basil-lsp` and `basil-dap`.

Project root: `editors/vscode-basil/`

## File Tree (create all)

editors/vscode-basil/
├─ package.json
├─ tsconfig.json
├─ src/
│  ├─ extension.ts               // activates LSP client and registers commands
│  ├─ lspClient.ts               // LanguageClient wiring for basil-lsp
│  └─ dapFactory.ts              // Debug adapter descriptor factory for basil-dap
├─ syntaxes/
│  └─ basil.monarch.json         // basic Monarch grammar for Basil
├─ language-configuration.json   // brackets, comments, auto-closing rules
├─ README.md
├─ .vscodeignore
└─ .gitignore

## package.json (requirements)

* Name: `basil-lang`
* Display name: `Basil`
* Publisher: `blackrush` (placeholder)
* Version: `0.0.1`
* Engines: `"vscode": "^1.89.0"` (or current stable)
* Categories: `["Programming Languages","Linters","Debuggers"]`
* Activation events:

  * `onLanguage:basil`
  * `onCommand:basil.run`
  * `onCommand:basil.build`
  * `onDebug:resolve:basil`
* Contributes:

  * `languages`: id `basil`, extensions `[".basil"]`, aliases `["Basil","basil"]`
  * `grammars`: Monarch grammar at `./syntaxes/basil.monarch.json` (language `basil`)
  * `configuration`: settings for executable paths:

    * `basil.lspExecutablePath` (string, default `basil-lsp`)
    * `basil.dapExecutablePath` (string, default `basil-dap`)
    * `basil.compilerExecutablePath` (string, default `basilc`)
    * `basil.compilerArgs` (array of strings)
  * `debuggers`:

    * `type`: `basil`
    * `label`: `Basil`
    * `program`: left blank (we’ll start `basil-dap` with a descriptor factory)
    * `languages`: `["basil"]`
    * `configurationAttributes.launch`: supports fields `program`, `cwd`, `args`, `stopOnEntry`
    * `variables`: none initially
  * `commands`:

    * `basil.run`: Run current file via `basilc`
    * `basil.build`: Build current file via `basilc`
  * `menus` (editor/title): buttons for Run / Build

## tsconfig.json

Standard VS Code extension tsconfig targeting ES2020, module commonjs, outDir `out`.

## src/extension.ts (implement)

* Read configuration values:

  * `lspPath = vscode.workspace.getConfiguration("basil").get<string>("lspExecutablePath","basil-lsp")`
  * `dapPath = vscode.workspace.getConfiguration("basil").get<string>("dapExecutablePath","basil-dap")`
  * `compilerPath = vscode.workspace.getConfiguration("basil").get<string>("compilerExecutablePath","basilc")`
  * `compilerArgs = vscode.workspace.getConfiguration("basil").get<string[]>("compilerArgs",[])`
* Start language client:

  * import `createLanguageClient` from `./lspClient`
  * `client = createLanguageClient(context, lspPath)` and `client.start()`
* Register commands:

  * `basil.run`: run the active `.basil` file (spawn `compilerPath` with `["run", <file>, ...compilerArgs]` or whichever CLI you already use)
  * `basil.build`: build to native (`["build", <file>, ...compilerArgs]` or your `bcc` pipeline)
  * Show output in a dedicated `Basil` output channel.
* Register debug adapter:

  * `vscode.debug.registerDebugAdapterDescriptorFactory("basil", new BasilDapFactory(dapPath))`
  * Implement “resolveDebugConfiguration” to default `program` to `${file}` when missing.

## src/lspClient.ts (implement)

* Use `vscode-languageclient/node`.
* Server options: spawn `lspPath` with stdio.
* Client options: document selector `{ language: "basil", scheme: "file" }`
* Synchronize: watch `**/*.basil`, and optionally `basil-lsp.toml`
* Return a `LanguageClient` instance.

## src/dapFactory.ts (implement)

* Export `class BasilDapFactory implements vscode.DebugAdapterDescriptorFactory`
* In `createDebugAdapterDescriptor`, spawn `dapPath` as a stdio server and return `DebugAdapterExecutable` or `DebugAdapterServer` (stdio is fine with `DebugAdapterExecutable`).
* Add `dispose` no-op.

## syntaxes/basil.monarch.json (starter grammar)

Include:

* Tokenizer with:

  * keywords: `["PRINT","INPUT","IF","THEN","ELSE","END","FOR","TO","STEP","NEXT","WHILE","WEND","DO","LOOP","FUNC","SUB","RETURN","DIM","AS","STRING","INTEGER","DOUBLE","BOOLEAN","TRUE","FALSE","REM","LET"]` (adjust to your Basil set)
  * comments: `REM ...` and `' ...` to end of line
  * strings: double-quoted with escapes
  * numbers: integers and floats
  * operators: `=`, `+`, `-`, `*`, `/`, `<>`, `<=`, `>=`, `<`, `>`
* Simple bracket/paren rules

## language-configuration.json

* Comments: lineComment: `'`
* Brackets: `[]`, `{}`, `()`
* AutoClosingPairs: quotes, brackets, parens
* WordPattern: reasonable regex for identifiers

## README.md

* What this extension does
* Requirements: install `basil-lsp`, `basil-dap`, and `basilc` on PATH (or set config paths)
* Settings table: the three paths + compiler args
* “Getting Started”:

  1. Open a folder with `.basil` files
  2. Install Basil toolchain and ensure on PATH
  3. Create `.vscode/launch.json` with the sample config below
  4. Open a `.basil` file; you should see diagnostics from LSP
* Example `launch.json`:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "basil",
      "request": "launch",
      "name": "Basil: Launch",
      "program": "${file}",
      "cwd": "${workspaceFolder}",
      "stopOnEntry": false
    }
  ]
}
```

* Commands:

  * Basil: Run (runs current file)
  * Basil: Build (builds current file)

## .vscodeignore

* Exclude: `src/**`, `tsconfig.json`, local node_modules bins; include `out/**`, `syntaxes/**`, `language-configuration.json`, `README.md`, `package.json`.

## .gitignore

* Standard Node/VS Code ignores: `node_modules/`, `out/`, `.vscode/`, `.DS_Store`

## Implementation notes and constraints

* Do not hardcode OS-specific executable names; assume `basil-lsp`, `basil-dap`, and `basilc` are resolvable from PATH unless overridden by settings.
* Use `vscode.workspace.openTextDocument` and `vscode.window.activeTextEditor?.document.fileName` to resolve the current file for Run/Build.
* Make sure the language client restarts if `basil.lspExecutablePath` changes (listen to `onDidChangeConfiguration` and stop/start).
* Guard spawns with try/catch and show a helpful error if `ENOENT`.
* Prefer `stdio` transports for both LSP and DAP.

## Acceptance criteria

* Opening a `.basil` file activates the extension, starts `basil-lsp`, and shows diagnostics.
* “Run” command executes the active `.basil` with the configured compiler, writing output to the Basil channel.
* “Build” command compiles successfully (or surfaces compiler errors in the output channel).
* Debugging with the sample `launch.json` launches `basil-dap`, sets/clears breakpoints, hits a breakpoint, and shows stack/scopes/variables.
* Works on Windows/macOS/Linux.

## Optional polish (nice to have)

* Status bar item for Basil mode (line/col already from VS Code; consider showing current VM state when debugging).
* Contribution for snippets: `print`/`if`/`for` templates.
* Add file icon theme contribution later.

Please generate all files with valid TypeScript code (no TODO placeholders), compile them (`npm run compile` or `vsce package` path if included), and ensure the extension runs in VS Code’s Extension Development Host.
