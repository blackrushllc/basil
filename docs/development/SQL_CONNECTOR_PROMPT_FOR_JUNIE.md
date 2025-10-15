# Junie Pro task: Basil **SQL Connectors – Phase 1** (MySQL & Postgres, RDS-friendly)

## Context

Basil already has SQLite and feature-gated objects. We want network SQL connectors that are:

* **Easy**: simple DSNs, pooled connections, JSON/arrays back.
* **Safe**: parameterized queries to avoid injection.
* **Portable**: works for MySQL (incl. Aurora/Percona/MariaDB) and PostgreSQL (incl. RDS/Aurora).
* **RDS-friendly**: TLS by default; allow custom CA.

**Process rules**

* Do **not** perform git actions (no branches/PRs/commits).
* **Must include runnable examples under `/examples/` and end-user docs** (acceptance depends on this).
* No tests required in this pass.

## Crate & features

Create a new crate: **`basil-objects-sql/`** with features:

* `obj-sql-mysql`     → enables MySQL connector
* `obj-sql-postgres`  → enables Postgres connector
* umbrella `obj-sql = ["obj-sql-mysql","obj-sql-postgres"]`
* `obj-all` includes obj-sql-mysql and obj-sql-postgres.

Wire via workspace dependencies (forward slashes). Register objects with the global registry **only** when features are enabled.

## Runtime & dependencies (Rust)

* Reuse the **global Tokio runtime** (lazy `once_cell`) used by other async mods.
* Use **SQLx** with `runtime-tokio`, `tls-rustls`:

    * `sqlx-mysql` (for MySQL/Aurora/Percona/MariaDB)
    * `sqlx-postgres`
    * `sqlx` core features: `macros` not required; dynamic queries are fine.
* TLS: **rustls** only (avoid native-tls/OpenSSL).
* Optional: `serde_json` for JSON assembling.

## Objects & API (Basil surface)

### 1) `DB_MYSQL`

Constructor (either DSN or discrete fields; DSN preferred):

```
DIM db@ AS DB_MYSQL("mysql://user:pass@host:3306/dbname?ssl-mode=REQUIRED")
```

Alternative constructor (optional):

```
DIM db@ AS DB_MYSQL()
db@.Connect$("host$", port%, "user$", "pass$", "dbname$", "ssl-mode$")
```

**Properties (get/set)**

* `PoolMax%` (default 5)
* `ConnectTimeoutMs%` (default 5000)
* `CommandTimeoutMs%` (default 30000)
* `TlsMode$` = `"DISABLED" | "PREFERRED" | "REQUIRED"` (default `"REQUIRED"` for RDS safety)
* `RootCertPath$` (optional file path to a PEM CA for RDS/custom CA)
* `LastRowsAffected%` (read-only)
* `LastError$` (read-only)

**Methods**

* `Execute(sql$, params$[]?)` → rows_affected%
  (INSERT/UPDATE/DELETE/DDL; params are string array—driver performs type coercion)
* `Query$(sql$, params$[]?)` → json$
  (JSON array of row objects; e.g., `[{"id":1,"name":"A"}]`)
* `QueryTable$(sql$, params$[]?)` → String[]
  (Optional convenience: flattened CSV-like lines or a simple table encoding)
* **Transactions**

    * `Begin()` → ok%
    * `Commit()` → ok%
    * `Rollback()` → ok%

**Notes**

* Parameter placeholders: `?` for MySQL.
* Named params not required in Phase 1.

### 2) `DB_POSTGRES`

Constructor:

```
DIM db@ AS DB_POSTGRES("postgres://user:pass@host:5432/dbname?sslmode=require")
```

Or:

```
DIM db@ AS DB_POSTGRES()
db@.Connect$("host$", port%, "user$", "pass$", "dbname$", "sslmode$")
```

**Properties (same semantics as MySQL, names reused)**

* `PoolMax%`, `ConnectTimeoutMs%`, `CommandTimeoutMs%`
* `TlsMode$` = `"disable" | "prefer" | "require"` (string mapped accordingly)
* `RootCertPath$`
* `LastRowsAffected%`, `LastError$`

**Methods (same shape as MySQL)**

* `Execute(sql$, params$[]?)` → rows_affected%
* `Query$(sql$, params$[]?)` → json$
* `QueryTable$(sql$, params$[]?)` → String[]
* `Begin()`, `Commit()`, `Rollback()`

**Notes**

* Parameter placeholders for Postgres: `$1`, `$2`, …
  Accept `params$[]` in order and bind positionally.

## Behavior & error handling

* Build a connection **pool** (SQLx) on first use; pool size from `PoolMax%`.
* **TLS on by default** (RDS-friendly); if `RootCertPath$` is provided, load and use it.
* All driver/POOL/SQL errors → **Basil exceptions** with clear messages:

    * `SQL(MySQL) ConnectFailed: <reason>`
    * `SQL(Postgres) QueryFailed: <code> <message>`
    * Include server code/class when available; **never** include passwords/DSNs in error text.
* On success, set `LastRowsAffected%` for Execute/DDL.

## Docs (must include)

Create **`docs/guides/SQL.md`** (single page covering both connectors):

* **Supported servers**: MySQL/Aurora/Percona/MariaDB; Postgres/Aurora Postgres.
* **Connection strings** (DSN examples):

    * MySQL: `mysql://user:pass@host:3306/db?ssl-mode=REQUIRED`
    * Postgres: `postgres://user:pass@host:5432/db?sslmode=require`
* **RDS/Aurora TLS**:

    * Default to TLS; how to set `RootCertPath$` with AWS RDS CA bundle.
* **Parameters & SQL injection**:

    * Always use params; show MySQL `?` vs Postgres `$1` examples.
* **Transactions**: `Begin/Commit/Rollback`.
* **Timeouts/pooling**: how `PoolMax%`, `ConnectTimeoutMs%`, `CommandTimeoutMs%` affect behavior.
* **Error handling** with `TRY/CATCH` (copyable snippets).
* **Feature flags & build**:

    * `cargo build --features obj-sql` or specific (`obj-sql-mysql`, `obj-sql-postgres`).

## Examples (must include; runnable)

Place under `/examples/`:

1. **`sql_mysql_quickstart.basil`**

```
REM MySQL quickstart (adapt host/user/pass/db)
LET dsn$ = "mysql://user:pass@localhost:3306/test?ssl-mode=DISABLED"
DIM db@ AS DB_MYSQL(dsn$)

TRY
  PRINTLN db@.Execute("CREATE TABLE IF NOT EXISTS t (id INT PRIMARY KEY AUTO_INCREMENT, name VARCHAR(50))")
  PRINTLN db@.Execute("INSERT INTO t (name) VALUES (?)", ["Alice"])
  PRINTLN db@.Execute("INSERT INTO t (name) VALUES (?)", ["Bob"])
  PRINTLN db@.Query$("SELECT id, name FROM t WHERE name LIKE ?", ["A%"])
CATCH e$
  PRINTLN "MySQL error: ", e$
END TRY
```

2. **`sql_postgres_quickstart.basil`**

```
REM Postgres quickstart (adapt host/user/pass/db)
LET dsn$ = "postgres://user:pass@localhost:5432/test?sslmode=disable"
DIM db@ AS DB_POSTGRES(dsn$)

TRY
  PRINTLN db@.Execute("CREATE TABLE IF NOT EXISTS t (id SERIAL PRIMARY KEY, name TEXT)")
  PRINTLN db@.Execute("INSERT INTO t (name) VALUES ($1)", ["Carol"])
  PRINTLN db@.Execute("INSERT INTO t (name) VALUES ($1)", ["Dave"])
  PRINTLN db@.Query$("SELECT id, name FROM t WHERE name LIKE $1", ["C%"])
CATCH e$
  PRINTLN "Postgres error: ", e$
END TRY
```

3. **`sql_txn_demo.basil`**

```
REM Transaction demo (either connector)
LET dsn$ = "mysql://user:pass@localhost:3306/test?ssl-mode=DISABLED"
DIM db@ AS DB_MYSQL(dsn$)

TRY
  db@.Begin()
  PRINTLN db@.Execute("INSERT INTO t (name) VALUES (?)", ["Txn1"])
  PRINTLN db@.Execute("INSERT INTO t (name) VALUES (?)", ["Txn2"])
  db@.Commit()
  PRINTLN db@.Query$("SELECT COUNT(*) AS cnt FROM t")
CATCH e$
  PRINTLN "SQL error: ", e$
  db@.Rollback()
END TRY
```

4. **`sql_rds_tls_example.basil`**

```
REM RDS TLS example (show RootCertPath$ usage)
LET dsn$ = "postgres://user:pass@mydb.abcdefg.us-east-1.rds.amazonaws.com:5432/app?sslmode=require"
DIM db@ AS DB_POSTGRES(dsn$)
db@.RootCertPath$ = "rds-ca-root.pem"   REM download AWS RDS CA bundle
PRINTLN db@.Query$("SELECT version()", [])
```

## Implementation notes (guidance)

* Build a small internal layer to:

    * Parse/set DSN and options → `sqlx::PoolOptions`.
    * Inject TLS via rustls; if `RootCertPath$` provided, load it and configure connector.
    * Bind parameters from `params$[]` in order (stringly-typed; driver handles conversion).
    * Convert returned rows → JSON (string) with `serde_json::Value` maps; keep numbers as numbers when possible.
* Ensure **pool reuse** across calls; reconnect on transient errors.
* Respect `CommandTimeoutMs%` with `sqlx::Executor::fetch_*` timeouts (or a client-side timeout wrapper).

## Acceptance checklist

* [ ] Workspace builds with `--features obj-sql` and also with features off.
* [ ] `DB_MYSQL` and/or `DB_POSTGRES` register only when features enabled; `DESCRIBE` shows properties/methods.
* [ ] Examples compile and run against local DBs (or RDS) when credentials are valid.
* [ ] `docs/guides/SQL.md` exists and covers DSNs, TLS/RDS, params, transactions, pooling/timeouts, feature flags, and `TRY/CATCH`.
* [ ] Errors map to Basil exceptions with clear, non-sensitive messages.

