# Compiled Classes FAQ

> The syntax of the CLASS function is, for example:
>
> DIM user@ AS CLASS("my_class.basil");
>
> Clearly this is a dynamic creation of an object in Basil.
>
> My question is, how does the compiler handle this?  Would it work?  Since"my_class.basil" is clearly not a compiled program, what will happen when a compiled Basil program executes this instrution?

---

### Short answer
Yes, it works. `CLASS("my_class.basil")` compiles to a runtime class-instantiation instruction. When your compiled Basil program executes it, the VM dynamically loads the class from disk. If a `.basilx` (precompiled) exists it will be loaded; otherwise the `.basil` source is compiled on the fly and instantiated. The class’s top-level code runs once to initialize its public variables, and the handle is stored in your `@` variable.

### What the compiler emits
- The main program compiles normally. The expression `CLASS(filename$)` is not inlined; it becomes a special opcode (conceptually `NEW_CLASS`).
- Member access like `user@.Description$` and `user@.AddUser(...)` compile to member opcodes (conceptually `GETMEMBER`/`SETMEMBER`/`CALLMEMBER`).

### What happens at runtime
When execution reaches `DIM user@ AS CLASS("my_class.basil")`:
1. Resolve the file path
    - If you provide a relative name (e.g., `"my_class.basil"`), the runtime first tries the directory of the running script, then the current working directory.
    - Search order by extension:
        - If you supply no extension: try `name.basil`, then `name.basilx`.
        - If you supply `.basil`: try that file first, then `same-name.basilx`.
        - If you supply `.basilx`: load that exact file.
2. Load or compile the class
    - If a `.basilx` is chosen, it’s deserialized and loaded directly.
    - If a `.basil` source is chosen, it is parsed and compiled in-memory at runtime. (Current implementation does not auto-write a cached `.basilx`.)
3. Run the class’s top-level code
    - The VM executes the class file’s top-level statements in an inner VM to initialize its public variables (acts like a constructor).
4. Create an instance handle
    - An instance object is created that exposes the class’s public variables and functions. Your `user@` variable receives this handle.
5. Member access works through the handle
    - `user@.Description$` reads/writes the instance’s stored variable.
    - `user@.AddUser("Erik")` invokes the class’s function in that instance’s context.

### Errors you might see
- If no suitable file is found: `Runtime Error: Class file not found.`
- If you try to run a class file by itself as a program: you’ll get a message indicating classes aren’t directly executable (classes must be instantiated from a program).
- Accessing a missing property or method yields an unknown member error.

### Performance and deployment notes
- If you ship only `.basil` class files, the first instantiation will pay a runtime compilation cost. If you want faster startup and predictable deployment, precompile and distribute `.basilx` files and reference them (or rely on the search order to find the `.basilx`).
- If you instantiate the same `.basil` class multiple times in one run, it will be compiled each time unless you supply a `.basilx`.

### Practical example
```basil
DIM user@ AS CLASS("my_class.basil")
PRINTLN "Initial description:", user@.Description$
LET user@.Description$ = "These are my favorite users."
user@.AddUser("Erik")
PRINTLN "User count:", user@.CountMyUsers%()
```
This will dynamically load or compile `my_class.basil`, run its top-level variable initializers, and let you call its public functions and access its public variables via `user@`.

### Summary
- The main program compiles once; class files are late-bound at runtime.
- The VM either loads a `.basilx` or compiles the `.basil` source in-memory, executes top-level init, and returns an instance handle.
- Your code then interacts with that instance through dot-notation member access and calls.


---
 
> What about Basil programs compuled to an executable (i.e. .EXE) with the bcc compiler?

---

### Short answer
Yes. An EXE built with `bcc` still executes `CLASS("...")` at runtime. The embedded VM performs late loading the same way: it looks for a `.basilx` (precompiled) or `.basil` (source) file on disk, loads (or compiles) it, runs the class’s top-level init code, and returns an instance handle.

### What changes when you ship an EXE
- The main program is packaged into a native executable together with the Basil VM.
- `CLASS(filename$)` remains a runtime instruction (conceptually `NEW_CLASS`) and is not inlined. The EXE must be able to find the class file(s) at runtime.
- If a `.basil` source is found, the parser/compiler inside the EXE compiles it in memory. If a `.basilx` is found, it is deserialized and loaded directly.

### Where the EXE looks for class files
Given `CLASS(path_or_name)`:
1. If you pass an absolute path or include a directory component (e.g., `"classes\\my_class.basilx"`), it tries that path directly.
2. If you pass just a name (no directory), the search order is:
    - The directory of the running script/program (for bundled EXEs this is typically the EXE’s directory).
    - The current working directory.
3. Extension handling:
    - No extension provided: try `name.basil`, then `name.basilx`.
    - `.basil` provided: try that file; if present it’s compiled. If not present, try the same base name with `.basilx`.
    - `.basilx` provided: load exactly that file.

If nothing is found you’ll get: `Runtime Error: Class file not found.`

### Performance and deployment recommendations
- Prefer shipping precompiled classes next to your EXE for fast startup and predictable behavior:
    - Precompile: `bcc -c my_class.basil` → produces `my_class.basilx`.
    - In code, reference `CLASS("my_class")` (no extension) or `CLASS("my_class.basilx")`.
- If you ship `.basil` source files, the EXE will compile them at runtime the first time you instantiate the class. Current implementation compiles in-memory and does not auto-write a cached `.basilx`.
- Keep classes in a known relative folder and reference them explicitly, for example:
    - Layout:
        - `MyApp.exe`
        - `classes\\my_class.basilx`
    - Code:
        - `DIM user@ AS CLASS("classes\\my_class.basilx")`
- Avoid relying on the process’s working directory (it can vary when launched from Explorer, services, etc.). Putting class files alongside the EXE or in a known subfolder is more reliable.

### Security/operational notes
- No internet access or installer is required at runtime; the EXE contains the VM. Only file read permissions are needed to access your `.basilx`/`.basil` files.
- If the class file is missing or unreadable (permissions, antivirus quarantine, path typos), instantiation will fail at the runtime point where `CLASS(...)` executes.

### Example
- Ship:
    - `App.exe`
    - `classes\\my_class.basilx`
- Code inside `App.exe`:
  ```basil
  DIM user@ AS CLASS("classes\\my_class.basilx")
  PRINTLN user@.Description$
  user@.AddUser("Erik")
  PRINTLN "Count:", user@.CountMyUsers%()
  ```
This will load `classes\\my_class.basilx` from disk at runtime and work identically to the script-run scenario.

### Summary
- Compiling to an EXE does not change `CLASS` semantics: loading/compiling of class files is still deferred to runtime.
- You must distribute the class files alongside the EXE (or point to an absolute path).
- For best results, precompile to `.basilx` and reference those files with stable relative paths.

---

> When we build executables using the bcc compiler, it contains the same bytecode VM and compiler that is in the basilc main interpreter program, and this is how it can generate and.run the basilx file?

--- 

### Short answer
Yes. An EXE built with `bcc` embeds the same Basil VM and the same parser/compiler components that the `basilc` interpreter uses. That’s why the EXE can both:
- Load and execute a precompiled `*.basilx` file, and
- If only `*.basil` source is present, parse and compile it in-memory at runtime, then execute it.

### What exactly is embedded
A `bcc`-produced executable links in:
- The Basil VM (bytecode interpreter) that executes bytecode programs.
- The compiler front-end used at runtime when needed:
  - `parse_basil(...)` (parser)
  - `compile_basil(...)` (compiler)
- The loader for precompiled bytecode: `deserialize_program(...)` for `*.basilx` files.

### How `CLASS("...")` works inside the EXE
When your EXE executes `CLASS(filename$)` (conceptually the `Op::NewClass` instruction):
1. It resolves the target file using the same search rules as the interpreter (relative to the program’s directory first, etc.).
2. It tries candidates based on the extension:
   - If it finds `name.basilx`: it deserializes it with `deserialize_program(...)` and runs it.
   - If it finds `name.basil`: it reads the source, runs `parse_basil(...)` then `compile_basil(...)`, and executes the result.
3. The class file’s top-level code is run once (in an inner VM) to initialize its public variables, and an instance handle is returned to your `@` variable.

### Does the EXE “generate a .basilx file” at runtime?
- It generates bytecode in memory from `.basil` source so it can run it immediately.
- Current implementation does not automatically write a cached `.basilx` file to disk when compiling at runtime; the compilation stays in-memory. If you want on-disk `*.basilx`, precompile them ahead of time and ship those.

### Practical recommendations
- For fastest startup and predictable deployment, precompile classes: `bcc -c my_class.basil` → `my_class.basilx`, and reference `CLASS("my_class")` or `CLASS("my_class.basilx")`.
- If you do ship `.basil` files, the EXE will compile them at the moment they are first instantiated.
- Keep class files next to the EXE or in a known subfolder and reference them by a stable relative path (e.g., `CLASS("classes\\my_class.basilx")`).

### Summary
- Your `bcc` EXE includes the Basil VM and the same parser/compiler as `basilc`.
- It can run `*.basilx` directly, or compile `*.basil` on the fly and run it.
- Runtime compilation is in-memory; it doesn’t auto-emit a `*.basilx` unless you precompile one yourself.



