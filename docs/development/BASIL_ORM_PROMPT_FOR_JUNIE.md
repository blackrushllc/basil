Haha, one last sprint before sleep 😄 — here’s a clean, copy-paste **Junie Pro prompt** to implement the **Basil ORM** on top of your SQL connectors. It’s feature-gated, dynamic (no codegen), uses parameterized SQL, and explicitly requires **examples and docs**.

---

# Junie Pro task: Basil **ORM** (dynamic Active Record on DB_MYSQL / DB_POSTGRES)

## Context

Basil already ships SQLite and network SQL connectors (`DB_MYSQL`, `DB_POSTGRES`). We want a lightweight **ORM** that provides:

* dynamic **models** defined at runtime (or introspected from the DB),
* a small **query builder**,
* **row objects** with `.Save()` / `.Delete()`,
* simple **relations** (has-many / belongs-to),
* JSON interop,
* transactions, and dialect awareness (MySQL & Postgres).

**Process rules**

* Do **not** perform git actions (no branches/PRs/commits).
* **You must include runnable examples under `/examples/` and an end-user doc.**
* No formal tests required in this pass.

## Crate & features

Create a new crate: **`basil-objects-orm/`** with features:

* `obj-orm` (core)
* `obj-orm-mysql` (enables MySQL adapter; depends on `obj-sql-mysql`)
* `obj-orm-postgres` (enables Postgres adapter; depends on `obj-sql-postgres`)
* umbrella `obj-orm-all = ["obj-orm","obj-orm-mysql","obj-orm-postgres"]`

Register the ORM object(s) with the global object registry **only** when features are enabled. Reuse the existing SQL connectors; do not duplicate connection logic.

## Public API (Basil surface)

### 1) Bootstrap

```
#USE ORM, DB_POSTGRES   ' or DB_MYSQL
DIM db@ AS DB_POSTGRES(dsn$)
DIM orm@ AS ORM(db@)              ' dialect auto-detected from db@
```

### 2) Model registration

Two paths:

* **Explicit**

  ```
  orm@.Model("users", ["id%","name$","email$","created_at$"], "id%")
  orm@.Model("posts", ["id%","user_id%","title$","body$","created_at$"], "id%")
  ```

* **Introspection**

  ```
  orm@.ModelFromTable$("users")   ' read columns & primary key from DB
  orm@.ModelFromTable$("posts")
  ```

> Column suffix mapping: `%`→integer, `$`→string/text, (none)→float/decimal; others map to `$` in Phase 1 (document this).

### 3) Relations

```
orm@.HasMany("users","posts","user_id%")          ' users.id -> posts.user_id
orm@.BelongsTo("posts","users","user_id%","id%")
```

### 4) Query builder

```
DIM q@ = orm@.Table("users")
q@.Where$("email$","=", "a@example.com").With$("posts").OrderBy$("id%","DESC").Limit%(10)
DIM rs@ = q@.Get()                                 ' ResultSet (iterable)
FOR EACH u@ IN rs@ : PRINT u@.Name$ : NEXT
```

Shortcuts:

```
DIM u@ = orm@.Table("users").Find%(42)             ' by primary key
DIM f@ = orm@.Table("users").First()               ' first row or null object
```

Supported builder methods (Phase 1):

* `Where$(col$, op$, val$)` (op in `"=","<>","<","<=",">",">=","LIKE"`)
* `AndWhere$()`/`OrWhere$()` (optional; or chain `Where` as AND)
* `OrderBy$(col$, dir$)` (`"ASC"|"DESC"`)
* `Limit%(n%)`, `Offset%(n%)`
* `With$(relation_name$)` eager-load has-many/belongs-to
* `Select$(cols$[])` (optional; default `*`)

### 5) Row objects (Active Record)

```
DIM u@ = orm@.New("users")
u@.Name$ = "Zoe" : u@.Email$ = "z@x.com"
u@.Save()                           ' INSERT

u@.Email$ = "zoe@example.com"
u@.Save()                           ' UPDATE (dirty columns only)

u@.Delete()                         ' DELETE by primary key
```

Relations on rows:

```
FOR EACH p@ IN u@.Posts() : PRINT p@.Title$ : NEXT        ' lazy
DIM p@ = u@.Posts().Create()
p@.Title$="Hello" : p@.Body$="…" : p@.Save()
```

### 6) JSON interop

* `u@.ToJson$()` → JSON object (column → value)
* `orm@.RowFromJson$("users", json$)` → row object
* `rs@.ToJson$()` → JSON array of rows

### 7) Transactions

```
orm@.Begin()
TRY
  ' multiple Save/Delete/Execute via ORM or raw DB
  orm@.Commit()
CATCH e$
  orm@.Rollback()
  PRINT "ORM txn failed: ", e$
END TRY
```

## Errors (Basil exceptions; user-facing)

* `ORM.ModelNotFound: <table> (pk=<value>)`
* `ORM.ValidationFailed: <table> (<field> required)`
* `ORM.QueryFailed(<dialect>): <code> <message>`
* `ORM.RelationMissing: <table>.<relation>`
* `ORM.UnknownColumn: <table>.<col>`

## Implementation details (Rust)

### Core objects

* `ORM` — holds:

    * reference to `db@` (either `DB_MYSQL` or `DB_POSTGRES`)
    * dialect info (`"mysql"` or `"postgres"`)
    * model registry: table → `ModelMeta { cols, pk, types, relations }`

* `Query` — built via `orm@.Table(name$)`:

    * collects predicates / order / limit / selected cols / eager relations
    * compiles to parameterized SQL:

        * MySQL placeholders: `?`
        * Postgres placeholders: `$1..$n`
    * executes via the underlying `DB_*` object’s `Query$()`; returns `ResultSet`

* `ResultSet` — iterable; exposes:

    * `Next()` (internal), enumeration via Basil `FOR EACH`
    * `Count%()`, `ToJson$()`, `First()` convenience

* `Row` — represents a record from a model:

    * property bag with typed access by suffix (mapped per ModelMeta)
    * dirty tracking; `.Save()` chooses INSERT vs UPDATE (PK presence)
    * `.Delete()` deletes by PK
    * relation accessors: `.Posts()` / `.User()` generated virtually via ModelMeta
    * `.ToJson$()`

### SQL compilation

* Safe quoting of identifiers per dialect (`"col"` vs `` `col` ``); avoid user-controlled identifier injection.
* Values always parameterized; never string-concat.
* `Where` chains combine with AND by default; allow `OrWhere` if included.

### Introspection (Phase 1 scope)

* **Postgres**: query `information_schema.columns` + `pg_constraint` for PK
* **MySQL**: query `information_schema.columns` + `information_schema.table_constraints` / `key_column_usage`
* Map DB types to Basil suffixes:

    * integer types → `%`
    * real/decimal → (none)
    * text/varchar/uuid/date/time/timestamp/json → `$` (document simplification)

### Eager loading `.With$()`

* For `HasMany`: collect PKs from main rows, `SELECT * FROM child WHERE fk IN (…)`, group by fk
* For `BelongsTo`: collect fks from main rows, `SELECT * FROM parent WHERE pk IN (…)`, map back
* Provide convenience counts: `.PostsCount%()` when eager loaded; otherwise lazy `.Posts().Count%()` by query.

### Transactions

* `ORM.Begin/Commit/Rollback` delegate to the underlying `DB_*` object.

### Diagnostics

* Include SQL **shape** in errors (e.g., `SELECT … WHERE email$ = ?`), but do **not** include parameter values.

## Docs (must include)

Create **`docs/integrations/orm/README.md`** with:

* What ORM provides; quickstart.
* Model registration (explicit vs `ModelFromTable$`).
* Suffix/type mapping table and limitations.
* Query builder cheatsheet.
* Row `.Save()`/`.Delete()` semantics and dirty tracking.
* Relations (has-many / belongs-to), eager loading `.With$()`.
* JSON interop.
* Transactions & error handling (`TRY/CATCH`).
* Dialect notes (MySQL vs Postgres placeholders).
* Feature flags & build:

    * `cargo build --features obj-orm-all` or specific (`obj-orm-mysql`, `obj-orm-postgres`).

## Examples (must include; runnable)

Place under `/examples/`:

1. **`orm_quickstart_pg.basil`** (Postgres)

```
#USE DB_POSTGRES, ORM
DIM db@ AS DB_POSTGRES("postgres://user:pass@localhost:5432/app?sslmode=disable")
DIM orm@ AS ORM(db@)

orm@.ModelFromTable$("users")
orm@.ModelFromTable$("posts")
orm@.HasMany("users","posts","user_id%")
orm@.BelongsTo("posts","users","user_id%","id%")

DIM u@ = orm@.New("users")
u@.Name$="Alice": u@.Email$="alice@example.com": u@.Save()

DIM p@ = u@.Posts().Create()
p@.Title$="First": p@.Body$="Hello": p@.Save()

DIM rs@ = orm@.Table("users").With$("posts").OrderBy$("id%","DESC").Get()
FOR EACH row@ IN rs@ : PRINT row@.Name$, " posts=", row@.PostsCount%() : NEXT
```

2. **`orm_quickstart_mysql.basil`** (MySQL)

```
#USE DB_MYSQL, ORM
DIM db@ AS DB_MYSQL("mysql://user:pass@localhost:3306/app?ssl-mode=DISABLED")
DIM orm@ AS ORM(db@)

orm@.Model("users", ["id%","name$","email$"], "id%")

DIM u@ = orm@.New("users")
u@.Name$="Bob": u@.Email$="bob@example.com": u@.Save()

PRINT orm@.Table("users").Where$("name$","LIKE","B%").OrderBy$("id%","ASC").Get().ToJson$()
```

3. **`orm_relations.basil`**

```
#USE DB_POSTGRES, ORM
DIM db@ AS DB_POSTGRES("postgres://user:pass@localhost:5432/app?sslmode=disable")
DIM orm@ AS ORM(db@)
orm@.ModelFromTable$("users")
orm@.ModelFromTable$("posts")
orm@.HasMany("users","posts","user_id%")
orm@.BelongsTo("posts","users","user_id%","id%")

DIM u@ = orm@.Table("users").First()
FOR EACH p@ IN u@.Posts() : PRINT p@.Title$ : NEXT
```

4. **`orm_transactions.basil`**

```
#USE DB_MYSQL, ORM
DIM db@ AS DB_MYSQL("mysql://user:pass@localhost:3306/app?ssl-mode=DISABLED")
DIM orm@ AS ORM(db@)

TRY
  orm@.Begin()
  DIM u@ = orm@.New("users")
  u@.Name$="Txn": u@.Email$="txn@example.com": u@.Save()
  orm@.Commit()
CATCH e$
  PRINT "ORM txn error: ", e$
  orm@.Rollback()
END TRY
```

## Acceptance checklist

* [ ] Workspace builds with `--features obj-orm-all` and with features off.
* [ ] `ORM` registers only when enabled; `DESCRIBE orm@` shows methods; `DESCRIBE` on `Row`/`Query`/`ResultSet` shows useful members.
* [ ] Examples compile and run against real DBs (given valid credentials).
* [ ] `docs/integrations/orm/README.md` exists and matches the implemented surface.
* [ ] Errors map to Basil exceptions with helpful messages; no secrets leaked.

## Follow-ups (note in README, not part of this task)

* Pagination helper (`Paginate%` returning page object), soft deletes, timestamps (`CreatedAt$`, `UpdatedAt$`).
* Many-to-many, composite PKs, validations API, migrations helper.
* Rich type mapping (UUID/JSONB/DATE ↔ Basil types or helpers).

---

That’s it — drop this in Junie and then go enjoy those podcasts 🎧🌙
