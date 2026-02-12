# BTC_MINER (obj-btcminer)

The `obj-btcminer` module provides objects for monitoring and orchestrating Bitcoin ASICs.

## Objects

### BTC_MINER_MGR@

Manages a collection of ASICs and provides snapshot capabilities.

#### Methods
- `ADD_ASIC%(name$, kind$, host$, user$, pass$)`: Adds an ASIC to the manager. Returns 1 on success, 0 on failure.
- `REMOVE_ASIC%(name$)`: Removes an ASIC from the manager.
- `SNAPSHOT@()`: Takes a snapshot of all registered ASICs. Returns a `Dict` with the following structure:
  - `timestamp%`: Unix timestamp
  - `asics@`: `List` of ASIC snapshots
  - `alerts@`: `List` of alert messages
  - `db_ok%`: Boolean (1/0) indicating if DB logging is okay
- `PROFILES@()`: Returns a `List` of built-in profile names.

### BTC_ASIC@

Represents an individual ASIC.

#### Constructor
- `BTC_ASIC@(kind$, host$, user$, pass$)`

#### Methods
- `INFO@()`: Returns a `Dict` with basic info (model, firmware).
- `STATS@()`: Returns a `Dict` with normalized stats (hashrate, temperature, etc.).
- `REBOOT%()`: Reboots the ASIC.
- `LAST_ERR$()`: Returns the last error message.

### BTC_ALERTS@

Evaluates alert rules against snapshots.

#### Methods
- `RULE_TEMP_MAX%(celsius#)`: Sets max temperature alert threshold.
- `RULE_HASHRATE_MIN%(ghs#)`: Sets min hashrate alert threshold.
- `RULE_REJECT_RATE_MAX%(pct#)`: Sets max reject rate threshold.
- `EVALUATE@(snapshot@)`: Evaluates a snapshot against the rules. Returns a `List` of alert strings.

### BTC_LOG_SQLITE@

Logs snapshots to a SQLite database.

#### Constructor
- `BTC_LOG_SQLITE@(path$)`

#### Methods
- `INIT%()`: Initializes the database schema.
- `WRITE_SNAPSHOT%(snapshot@)`: Writes a snapshot to the database.
- `LAST_ERR$()`: Returns the last error message.

## Example

```basil
DIM mgr@ AS BTC_MINER_MGR()
mgr@.ADD_ASIC%("S19-01", "generic-http-json", "192.168.1.100", "root", "root")

DIM snap@
LET snap@ = mgr@.SNAPSHOT@()

PRINT "Miner: "; snap@{"asics"}(0){"name"}
PRINT "Hashrate: "; snap@{"asics"}(0){"stats"}{"hashrate_ghs"}; " GH/s"
```
