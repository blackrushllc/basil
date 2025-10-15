# Basil SQL Connectors (MySQL and PostgreSQL)

This guide covers the network SQL connectors available in Basil: DB_MYSQL and DB_POSTGRES. They provide pooled connections, parameterized queries, JSON/table outputs, and RDS-friendly TLS by default.

Supported servers
- MySQL family: MySQL, Amazon Aurora MySQL, Percona, MariaDB
- PostgreSQL family: PostgreSQL, Amazon Aurora PostgreSQL

Connection strings (DSN)
- MySQL: mysql://user:pass@host:3306/dbname?ssl-mode=REQUIRED
- Postgres: postgres://user:pass@host:5432/dbname?sslmode=require

Quick constructor examples
- MySQL
  DIM db@ AS DB_MYSQL("mysql://user:pass@localhost:3306/test?ssl-mode=DISABLED")
- Postgres
  DIM db@ AS DB_POSTGRES("postgres://user:pass@localhost:5432/test?sslmode=disable")

Alternative Connect$ constructor
- MySQL
  DIM db@ AS DB_MYSQL()
  db@.Connect$("host$", 3306, "user$", "pass$", "dbname$", "REQUIRED")
- Postgres
  DIM db@ AS DB_POSTGRES()
  db@.Connect$("host$", 5432, "user$", "pass$", "dbname$", "require")

TLS and RDS
- TLS is on by default (REQUIRED/require) for safety.
- You can provide a custom CA (e.g., AWS RDS root CA bundle) via RootCertPath$.
  Example:
  LET dsn$ = "postgres://user:pass@mydb.abcdefg.us-east-1.rds.amazonaws.com:5432/app?sslmode=require"
  DIM db@ AS DB_POSTGRES(dsn$)
  db@.RootCertPath$ = "rds-ca-root.pem"

Parameters and SQL injection
- Always use parameters to avoid SQL injection.
- Placeholders:
  - MySQL: ?
  - Postgres: $1, $2, ...
- Example (MySQL):
  PRINTLN db@.Execute("INSERT INTO t(name) VALUES (?)", ["Alice"])  
  PRINTLN db@.Query$("SELECT id, name FROM t WHERE name LIKE ?", ["A%"])
- Example (Postgres):
  PRINTLN db@.Execute("INSERT INTO t(name) VALUES ($1)", ["Alice"])  
  PRINTLN db@.Query$("SELECT id, name FROM t WHERE name LIKE $1", ["A%"])

Transactions
- Methods: Begin(), Commit(), Rollback()
- Example:
  db@.Begin()
  PRINTLN db@.Execute("INSERT INTO t(name) VALUES (?)", ["Txn1"])  
  PRINTLN db@.Execute("INSERT INTO t(name) VALUES (?)", ["Txn2"])  
  db@.Commit()

Timeouts and pooling
- PoolMax% (default 5): maximum pooled connections
- ConnectTimeoutMs% (default 5000): connection acquisition timeout
- CommandTimeoutMs% (default 30000): per-command client-side timeout

Error handling
- Errors are thrown as Basil exceptions with clear messages like:
  - SQL(MySQL) ConnectFailed: <reason>
  - SQL(Postgres) QueryFailed: <reason>
- Wrap in TRY/CATCH:
  TRY
    PRINTLN db@.Query$("SELECT 1", [])
  CATCH e$
    PRINTLN "SQL error: ", e$
  END TRY

Feature flags and building
- Enable both connectors umbrella: cargo build --features obj-sql
- Or specific:
  - MySQL only: cargo build --features obj-sql-mysql
  - Postgres only: cargo build --features obj-sql-postgres

Notes
- JSON output: Query$ returns a JSON array of row objects, with numbers preserved when possible.
- QueryTable$: returns a simple CSV-like list of lines (first line is header).
- RDS TLS: Provide RootCertPath$ with the downloaded AWS RDS CA bundle when needed.
