# OBJECTS.md

Abstract: This is going to be a new concept for our BASIC interpreter. 

As you know, BASIL is a BASIC interpreter written in Rust.

We're going add a new data type: Built-in Objects. 

Right now I have Strings, Integers and floats where the identifiers (variable
names) indicate the data type with a dollar sign for String (i.e. MyString\$) or a percent sign for integer 
(i.e. MyInteger%) and indenitifiers with no special character are considered to be floating point numbers. 

I have also added Array support for String, Integer and Float data types by using parenthesis to indicate the subscripts (up to 4 dimensions) and they are initialized with the DIM statement, i.e. DIM MyStringArray(10). 

I want to introduce a new data type which is a built-in object that can be instantiated much like the DIM statement is 
used to create arrays, but using a new keyword or perhaps retasking DIM or LET to be used in the 
instantiation of a new object identified by a name. I want to have these objects stored in a library of 
objects in my Rust project. I want to be able to choose which objects are included in the build. When these 
objects have been compiled into the language, 
I want the native BASIC to be able to instantiate them into a variable that has a special symbol i.e. MyObject@,
Conn@, etc. like how $ is used for strings and % is used for integers. 

I want the native BASIC to be able to set properties, call methods, and have results returned from
these objects.. I want the native BASIC to be able to dump a description of an object to show it's public properties and
methods, which could be useful to the programmer. 

An example of such a library object would be a file IO object, which
can open and work with files on the device, and perhaps have other useful methods related to working with files. 

Another example of such a library object would be a PDO database connector, which can connect to a data source, send queries,
and return results from a database.

All non-core language elements will be added to the language as extensions and only 
core syntax changes will be made to the interpreter code from this point onwards..

## Design


# 1) Surface design (what BASIC sees)

* **Type marker:** pick one symbol for “object” variables (e.g., `@`), parallel to `$` and `%`.
  Example: `DIM f@ AS FILE`, `DIM db@ AS PDO`.
* **Instantiation:** two good options:

    * `DIM f@ AS FILE("foo.txt", FOR_READ)` (DIM-as-ctor)
    * or `LET f@ = NEW FILE("foo.txt", FOR_READ)` (explicit NEW)
* **Member access:** `f@.Open()`, `f@.Size`, `f@.Write("hi")`, `result$ = db@.Query("SELECT ...")`
* **Introspection:** `DESCRIBE f@` or function form `PRINT DESCRIBE$(f@)` to dump methods/properties.
* **Opt-in modules:** `#USE FILE`, `#USE PDO` or `OPTION MODULE FILE, PDO` (compile-time hint; see §3).

This gives you readable programs, consistent with your `$`/`%` conventions, and leaves arrays of primitives as-is. (You
can add arrays of objects later via `DIM arr@(10) AS FILE`, but defer at first.)

# 2) Project layout (one place for everything)

```
basil/
├─ basil-ast/            # AST nodes (add ObjectTypeName, MemberCall, etc.)
├─ basil-lexer/          # tokens: NEW, AS, DESCRIBE, dot, '@' suffix
├─ basil-parser/         # parses DIM/NEW/obj@.call
├─ basil-compiler/       # lowers to object-aware IR/bytecode
├─ basil-vm/             # VM ops + object runtime
├─ basil-objects/        # NEW: built-ins library (feature-gated)
│  ├─ src/
│  │  ├─ lib.rs          # registry + shared traits + derive macro re-exports
│  │  ├─ file.rs         # feature "obj-file"
│  │  ├─ pdo.rs          # feature "obj-pdo"
│  │  └─ ...
│  └─ Cargo.toml         # defines features: obj-file, obj-pdo, objects-full
└─ basil-cli/            # front-end; honors #USE hints and shows describe dumps nicely
```

One crate (`basil-objects`) holds all object types, each behind a **Cargo feature**. Core interpreter depends on this
crate but activates zero features by default (tiny core). You can also split each object into its own crate later if you
prefer stricter isolation.

# 3) Build-time inclusion (clean & conventional)

* **Cargo features** in `basil-objects`:

    * `obj-file`, `obj-pdo`, … and a convenience `objects-full = ["obj-file","obj-pdo", ...]`
* **Conditional compilation** (`#[cfg(feature = "obj-file")]`) exposes a factory into a **global registry** (see §4).
* **BASIC meta directives** `#USE FILE, PDO`:

    * The compiler *doesn’t* toggle Cargo; instead it:

        * validates that requested modules are present in the runtime registry,
        * errors nicely if unavailable (“FILE not compiled in; enable feature `obj-file`”).
    * For your own builds, you toggle features in `Cargo.toml`/`--features`. For end users, your “distribution
      profiles” (e.g., *Safe*, *Full*, *Embedded*) map to feature sets.

This keeps the workflow standard (Cargo features) while giving BASIC authors discoverable feedback via `#USE`.

# 4) Object model (one trait, one registry)

* **Core trait (conceptually):** each object type implements a small, uniform interface:

    * `type_name()`
    * `construct(args: &[Value]) -> ObjectInstance`
    * `get_prop(name) -> Value`
    * `set_prop(name, Value)`
    * `call(method_name, &[Value]) -> Value`
    * `describe() -> ObjectDescriptor` (methods, props, doc)
* **Instance representation:** store in VM as a **handle** (e.g., `Rc<RefCell<dyn BasilObject>>`).
  Your `Value` enum grows a variant like `Object(Handle)`.
* **Global registry:** at program start, compile all enabled object factories into a map:

    * `Registry: type_name -> Factory` (populated only for compiled-in features).
    * Parser/lowerer can resolve `AS FILE` to a type-id; VM uses the factory to `NEW`.

This gives you two levels of lookup: a one-time type resolution at compile time, and fast method/property dispatch at
runtime.

# 5) VM & bytecode (minimal, fast, stable)

Add a tiny set of **generic** ops—no per-object opcodes needed:

* `NEW_OBJ type_id, argc` → pushes `Object`
* `GETPROP prop_id` / `SETPROP prop_id`
* `CALLMETHOD method_id, argc`
* `DESCRIBE_OBJ` → printable string or a structured value

Where do `*_id`s come from? The compiler interns member names per type (method/property tables from `describe()` at
compile time or the first time they’re seen), caches indices, and emits ids. Keep a **slow-path** (string lookup) for
dynamic cases and a **fast-path** (cached ids) for hot calls.

# 6) Reflection & documentation (built-in and free)

Give each object a machine-readable descriptor:

* **Descriptor contents:** public methods (name, arity, param names/types, returns), properties (name,
  readable/writable, type), summary docs, version.
* **Derive macro:** a `#[derive(BasicObject)]` proc-macro in `basil-objects` can auto-generate:

    * the trait impl,
    * the registration glue,
    * and the descriptor table from simple annotations.
* **Runtime dump:** `DESCRIBE f@` prints a consistent table; `HELP "FILE"` prints the type’s descriptor even without an
  instance.

This unlocks good errors, editor hints, and self-documenting BASIC.

# 7) Types & interop (keep it BASIC-y)

* **Arguments/returns:** use your existing `Value` variants (Float, Int, String, Array, Object). Objects may return
  arrays or iterators; if you add an **enumerator protocol** later (e.g., `FOR EACH row IN rs@`), you’re ready.
* **Coercions:** centralize conversions (e.g., when a method expects Int but gets Float 3.0).
* **Errors:** methods return either `Value` or a **BasilError** that the VM maps to BASIC exceptions (with
  object/type/method context in the message).

# 8) Security & sandboxing (important for FILE/PDO)

* **Compile-time capability flags:** features like `obj-file` gate the whole type.
* **Runtime policy hooks:** allow a host policy (passed to VM) to veto certain operations:

    * `File.open` restricted to certain directories; `PDO` only to DSNs allowed by host.
* **Determinism toggles:** for embedded or teaching mode, you can disable wall-clock/file/network objects entirely.

# 9) Testing strategy (fast to trust)

* **Golden tests** for parsing (`DIM f@ AS FILE("x")` → AST).
* **Lowering tests** assert bytecode sequences for member calls.
* **VM tests**: table-driven (operation, inputs → outputs/side effects).
* **Contract tests** for each object type, reusing the same harness (ensures consistent error messages, reflection,
  property bounds, etc.).

# 10) Migration & scope control

* Ship with 1–2 showcase objects first (e.g., `FILE`, `CLOCK`).
* Add `DESCRIBE TYPE "FILE"` early—even before full method coverage—so users can see progress.
* Defer **arrays of objects**, **inheritance**, **interfaces**, and **async**. You can add a lightweight **iterator
  protocol** later without breaking the model.

# 11) Example life cycle (end-to-end, concept only)

1. Build: `--features obj-file,obj-pdo`
2. BASIC:

   ```
   #USE FILE, PDO
   DIM f@ AS FILE("out.txt")
   f@.WriteLine("Hello")
   DESCRIBE f@
   ```
3. Parser resolves `FILE` → type token; compiler emits `NEW_OBJ type(FILE), 1`, etc.
4. VM consults registry → constructs instance; `CALLMETHOD` hits fast table.

# 12) Why this works well

* **BASIC feel preserved:** single symbol suffix, simple dot calls.
* **Small core:** the VM learns only “object plumbing,” not domain features.
* **Composable & opt-in:** Cargo features decide footprint; `#USE` gives user-facing clarity.
* **Introspectable:** first-class reflection improves UX, docs, and error quality.
* **Future-proof:** the same mechanism can host timers, sockets, HTTP, RNG, UI stubs, etc.


