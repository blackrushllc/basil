# ORM object for Basil

This is the user-facing reference for the Phase 1 ORM shipped as a feature-gated object module.

- Crate: basil-objects-orm
- Features:
  - obj-orm (core surface)
  - obj-orm-mysql (enables MySQL adapter; depends on obj-sql-mysql)
  - obj-orm-postgres (enables Postgres adapter; depends on obj-sql-postgres)
  - obj-orm-all = ["obj-orm","obj-orm-mysql","obj-orm-postgres"]
- Included in umbrella `obj-all` via basil-objects.

## What it provides
- Dynamic models (explicit or introspected from DB)
- Query builder with Where$/OrderBy$/Limit%/Offset%/Select$
- Row objects with Save()/Delete() and ToJson$()
- Relations: HasMany / BelongsTo
- Transactions delegating to DB.Begin/Commit/Rollback
- JSON interop for rows and queries

Note: Query.Get() returns ARRAY<ORM_ROW> to allow `FOR EACH` loops (VM iterates arrays).
Use Query.ToJson$() to fetch results as JSON.

## Bootstrap
```
#USE ORM, DB_POSTGRES
DIM db@ AS DB_POSTGRES("postgres://user:pass@localhost:5432/app?sslmode=disable")
DIM orm@ AS ORM(db@)
```

## Model registration
```
orm@.Model("users", ["id%","name$","email$"], "id%")
' or
orm@.ModelFromTable$("users")
```

## Querying
```
DIM rows@ = orm@.Table("users").Where$("email$","=","a@example.com").OrderBy$("id%","DESC").Limit%(10).Get()
FOR EACH u@ IN rows@ : PRINTLN u@.Name$ : NEXT
PRINTLN orm@.Table("users").Limit%(3).ToJson$()
```

## Rows (Active Record)
```
DIM u@ = orm@.New("users")
u@.Name$="Alice" : u@.Email$="alice@example.com" : u@.Save()

u@.Email$="a@x.com" : u@.Save()

u@.Delete()
```

## Relations
```
orm@.HasMany("users","posts","user_id%")
orm@.BelongsTo("posts","users","user_id%","id%")

DIM u@ = orm@.Table("users").First()
FOR EACH p@ IN u@.Posts() : PRINTLN p@.Title$ : NEXT
```

## Transactions
```
TRY
  orm@.Begin()
  ' ... many operations ...
  orm@.Commit()
CATCH e$
  PRINTLN "ORM txn error:", e$
  orm@.Rollback()
END TRY
```

## Errors
- ORM.ModelNotFound: <table>
- ORM.ValidationFailed: <table> (<field> required)
- ORM.QueryFailed(<dialect>): <code> <message>
- ORM.RelationMissing: <table>.<relation>
- ORM.UnknownColumn: <table>.<col>

## Building
```
cargo build --features obj-orm-all
```
Or enable one adapter:
- `cargo build --features obj-orm-postgres`
- `cargo build --features obj-orm-mysql`
