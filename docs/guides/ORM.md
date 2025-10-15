# Basil ORM (Phase 1)

Lightweight dynamic Active Record for MySQL and Postgres built on Basil's DB connectors (DB_MYSQL, DB_POSTGRES).

What you get in Phase 1:
- Dynamic models (explicit or DB introspection)
- Small query builder with parameterization
- Row objects with Save()/Delete() and JSON interop
- Simple relations (has-many / belongs-to)
- Transactions delegating to the underlying DB object
- Dialect-aware identifier quoting and placeholders

Note: In Phase 1, Query.Get() returns an array of ORM_ROW objects to integrate with Basil's FOR EACH (which currently iterates arrays). Use Query.ToJson\$() to retrieve the JSON form of a result set.

## Quickstart

```
#USE ORM, DB_POSTGRES
DIM db@ AS DB_POSTGRES("postgres://user:pass@localhost:5432/app?sslmode=disable")
DIM orm@ AS ORM(db@)

orm@.ModelFromTable\$("users")
DIM rs@ = orm@.Table("users").Where\$("email\$","=","a@example.com").OrderBy\$("id%","DESC").Limit%(10).Get()
FOR EACH u@ IN rs@ : PRINT u@.Name\$ ; NEXT
```

## Model registration

Explicit:
```
orm@.Model("users", ["id%","name\$","email\$","created_at\$"], "id%")
```

Introspection:
```
orm@.ModelFromTable\$("users")
```

Suffix mapping (simplified in Phase 1):
- `%` -> integer types
- `\$` -> text, varchar, uuid, date/time, json
- none -> real/decimal
- others -> treated as `\$`

## Relations

```
orm@.HasMany("users","posts","user_id%")
orm@.BelongsTo("posts","users","user_id%","id%")
```

On a row:
```
DIM u@ = orm@.Table("users").First()
FOR EACH p@ IN u@.Posts() : PRINT p@.Title\$ : NEXT
```

## Query builder

```
DIM q@ = orm@.Table("users")
q@.Where\$("email\$","=","a@example.com").OrderBy\$("id%","DESC").Limit%(10)
DIM rows@ = q@.Get()   ' returns ARRAY<ORM_ROW>
PRINTLN q@.ToJson\$()
```

Supported methods:
- Where\$(col\$, op\$, val\$)
- OrderBy\$(col\$, dir\$)
- Limit%(n%), Offset%(n%)
- With\$(relation_name\$)   ' reserved; eager loading planned
- Select\$(cols\$[])
- Get() -> ARRAY<ORM_ROW>
- Find%(pk%) -> ORM_ROW
- First() -> ORM_ROW or null
- ToJson\$() -> JSON array

## Row objects

```
DIM u@ = orm@.New("users")
u@.Name\$ = "Zoe" : u@.Email\$ = "z@x.com" : u@.Save()

u@.Email\$ = "zoe@example.com" : u@.Save()

u@.Delete()
```

- Save(): INSERT when PK missing; UPDATE only dirty columns otherwise.
- Delete(): deletes by primary key.
- ToJson\$(): JSON object of column -> value.

## JSON interop
- Row.ToJson\$() -> JSON object
- Query.ToJson\$() -> JSON array of rows
- ORM.RowFromJson\$(table\$, json\$) -> row object

## Transactions

```
TRY
  orm@.Begin()
  ' ... multiple Saves or raw DB calls
  orm@.Commit()
CATCH e\$
  orm@.Rollback()
  PRINTLN "ORM txn failed: ", e\$
END TRY
```

## Errors
- ORM.ModelNotFound: <table>
- ORM.ValidationFailed: <table> (<field> required)
- ORM.QueryFailed(<dialect>): \<code> \<message> - surfaced from DB connector
- ORM.RelationMissing: \<table>.\<relation>
- ORM.UnknownColumn: \<table>.\<col>

## Dialect notes
- Postgres placeholders: \$1..\$n; MySQL: ?
- Identifiers are quoted per dialect: "col" vs `col`

## Feature flags & build

```
cargo build --features obj-orm-all
```
Or selectively:
- obj-orm-mysql
- obj-orm-postgres

Also available under umbrella obj-all via basil-objects.

## Limitations (Phase 1)
- Query.Get() returns ARRAY<ORM_ROW> (not a ResultSet object) to support FOR EACH.
- Eager loading with With\$() is stubbed; relations are accessible lazily on rows.
- Type mapping is simplified; many types map to string `\$`.
