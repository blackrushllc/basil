You are “Basil Junie”. Implement a new Basil feature object module named obj-btcminer (v1: ASIC monitoring/orchestration, logging, alerts). Target Windows + Linux (macOS ok if no platform-specific deps). Follow Basil conventions: small ergonomic APIs, return codes, errors mapped to Basil exceptions where appropriate, and include /examples + docs.

REPO CONTEXT (assume workspace):
- Basil is a BASIC-like language in Rust with type suffixes: \$ string, % integer, none float, @ object.
- Feature objects live under crates/obj-*/ and are feature-gated in the workspace.
- Prefer rustls for TLS if needed.
- Single global Tokio runtime is used for async mods.
- Errors should surface as Basil exceptions; also keep LAST_ERR\$() for objects where useful.

SCOPE (v1):
1) Provide Basil objects:
   A) BTC_MINER_MGR@
    - ctor: BTC_MINER_MGR@()
    - ADD_ASIC%(name\$, kind\$, host\$, user\$, pass\$)
    - REMOVE_ASIC%(name\$)
    - SET_POOLS%(name\$, pools@) ; pools@ is list of dicts {url\$, user\$, pass\$, priority%}
    - APPLY_POOLS%(name\$)
    - REBOOT_ASIC%(name\$)
    - START%(interval_ms%)
    - STOP%()
    - SNAPSHOT@() -> dict: {timestamp%, asics@, alerts@, db_ok%, last_err\$}
      B) BTC_ASIC@
    - ctor: BTC_ASIC@(kind\$, host\$, user\$, pass\$)
    - INFO@()
    - STATS@()
    - SET_POOLS%(pools@)
    - REBOOT%()
    - LAST_ERR\$()
      C) BTC_ALERTS@
    - ctor: BTC_ALERTS@()
    - SMTP_CONFIG%(host$, port%, user\$, pass\$, from\$, to\$)
    - WEBHOOK_CONFIG%(url\$)
    - RULE_TEMP_MAX%(celsius#)
    - RULE_HASHRATE_MIN%(ghs#)
    - RULE_REJECT_RATE_MAX%(pct#)
    - EVALUATE%(snapshot@)
      D) BTC_LOG_SQLITE@
    - ctor: BTC_LOG_SQLITE@(path\$)
    - INIT%()
    - WRITE_SNAPSHOT%(snapshot@)
    - LAST_ERR\$()

2) Rust architecture:
    - Create a core crate crates/btcminer-core with:
        - AsicAdapter trait (info, stats, set_pools, reboot)
        - Adapter registry by kind\$ string
        - Snapshot types (serde Serialize)
        - Alert evaluator logic
        - SQLite logger (behind feature db_sqlite)
    - Create crates/obj-btcminer that exposes the Basil bindings and objects, using core crate.

3) Adapters (start simple):
    - Implement “generic-http-json” adapter first:
        - Configurable endpoints via a small per-device config dict (or default endpoints).
        - For v1 you may hardcode one “example” endpoint set and clearly document it.
    - Stub vendor adapters (antminer/braiins/whatsminer) but keep them behind feature flags or TODO with safe placeholders.

4) Networking:
    - Use reqwest (rustls) for HTTP(S).
    - Handle auth basic/digest if easy; if not, start with basic auth and cookie session if needed.

5) SQLite schema:
    - tables: asics, samples, alerts
    - indexes on (asic_id, ts)
    - Store extra vendor fields in notes_json/payload_json.

6) Basil data mapping:
    - Return dictionaries/lists/objects that feel natural in Basil.
    - Keep numeric fields as floats/ints, strings as strings.

7) Examples:
    - Add /examples/btcminer_monitor.basil: registers 2 asics, sets pools, starts loop, writes sqlite, sends alerts.
    - Add /examples/btcminer_status_json.basil: prints SNAPSHOT@ as JSON (if there is a JSON encoder; otherwise format manually).

8) Docs:
    - Add docs/reference/obj-btcminer.md with all methods, argument shapes, sample pools@ structure, and sample snapshot output.

9) Quality:
    - No panics in runtime path.
    - Clear error messages; LAST_ERR\$ updated on failure.
    - Unit tests in core crate for alert evaluator and snapshot serialization.

Deliverables:
- All new crates + workspace wiring + feature flags.
- Examples + docs.
- A short CHANGELOG entry describing obj-btcminer v1.



ADDENDUM: Miner profiles (Antminer / WhatsMiner / Braiins OS) via “generic-http-json” profiles

Goal:
- Keep v1 moving by using the generic-http-json adapter as the primary implementation.
- Add vendor “profiles” as config templates that map vendor endpoints + JSON fields into our normalized stats model.
- Provide clear documentation and sample configs so users can add/adjust profiles without code changes.

A) Normalize the stats model (core)
Define a normalized stats struct in btcminer-core (serde Serialize), used across all adapters:
- model\$ (string)
- firmware\$ (string)
- uptime_s% (int)
- hashrate_ghs# (float)
- hashrate_ghs_5m# (float, optional)
- temp_max_c# (float)
- temp_avg_c# (float, optional)
- fan_rpm_avg% (int)
- fan_rpm_min% (int, optional)
- accepted% (int)
- rejected% (int)
- reject_rate_pct# (float, computed if possible)
- pool_url\$ (string, best-effort)
- pool_user\$ (string, best-effort)
- raw@ (dict/json) ; store full vendor payload

All device-specific / unknown fields go into raw@.

B) “Profile config” format
Create a profile configuration format that can be:
1) Embedded as JSON files in the repo (recommended for v1), AND
2) Passed as a Basil dictionary (optional but nice).

Suggested file location:
- crates/btcminer-core/profiles/*.json
  And docs:
- docs/reference/obj-btcminer-profiles.md

Profile JSON schema (v1):
```
{
"profile_name": "antminer_http_v1",
"kind": "generic-http-json",
"auth": { "type": "basic" | "cookie" | "none", "username_key": "user", "password_key": "pass" },
"endpoints": {
"info":  { "method": "GET", "path": "/cgi-bin/get_system_info.cgi" },
"stats": { "method": "GET", "path": "/cgi-bin/stats.cgi" }
},
"extract": {
"model": "jsonpath-like expression",
"firmware": "jsonpath-like expression",
"uptime_s": "expression",
"hashrate_ghs": "expression",
"temp_max_c": "expression",
"fan_rpm_avg": "expression",
"accepted": "expression",
"rejected": "expression",
"pool_url": "expression",
"pool_user": "expression"
},
"notes": "human-readable notes / caveats"
}
```
Important: Keep extraction simple for v1. If full JSONPath is too heavy, implement a minimal extractor:
- Dot paths: "foo.bar[0].baz"
- Fallback list: ["path1","path2","path3"] (first found wins)
- Basic transforms: to_int, to_float, max(list), avg(list), sum(list), multiply/divide
  If implementing a mini expression engine is too much, hardcode a few transforms and document limitations.

C) Provide 3 initial vendor profiles (best-effort)
Implement and include three “starter” profiles as templates:
1) Antminer profile template:
- Document that Antminer endpoints vary by model/firmware and may require web session auth.
- Provide a “likely” baseline profile with placeholders for endpoints/paths and include notes on how to sniff endpoints by using browser devtools.
- Include two variants if easy: basic-auth and cookie-session.

2) WhatsMiner profile template:
- Provide a baseline profile and notes that WhatsMiner often has its own API patterns.
- If you cannot reliably implement exact endpoints without a device, keep it as a template with TODO markers and explain how users should fill in the endpoint paths and extraction fields.

3) Braiins OS profile template:
- Provide a baseline profile for Braiins OS (best effort). If you can find stable endpoints quickly from public docs, implement them; otherwise provide a template with explicit instructions on how to adapt.
- Emphasize that Braiins OS tends to be more API-friendly; document how to verify endpoints.

Do NOT block the PR on perfect vendor correctness. The deliverable is:
- A working generic profile mechanism
- Vendor templates that are clearly labeled “starter” and easy to customize

D) Basil API additions for profiles
Extend BTC_MINER_MGR@ and BTC_ASIC@ to accept an optional profile:
- ADD_ASIC%(name\$, kind\$, host\$, user\$, pass\$[, profile_name\$])
- or ADD_ASIC_WITH_PROFILE%(name\$, host\$, user\$, pass\$, profile_name$)
  Also add:
- PROFILES@() -> list of built-in profile names
- LOAD_PROFILE%(profile_name$) -> returns profile dict (optional)
- SET_PROFILE%(name$, profile_name\$) (assign profile to device)

E) Documentation requirements (profile configs)
Create docs/reference/obj-btcminer-profiles.md with:
1) What profiles are and why they exist.
2) Full schema explanation with examples.
3) How to create a new profile:
    - start from vendor template
    - set endpoints
    - set extract mappings
    - test via example script
4) “Troubleshooting” section:
    - authentication failures
    - endpoint not found
    - JSON shape changed
    - missing fields (fallback behavior)
5) Include three sample profile JSON blocks (Antminer/WhatsMiner/Braiins) as code blocks in docs.
6) Include a “Field mapping cookbook”:
    - how to compute hashrate_ghs from vendor values
    - how to select max temp from chains/boards
    - how to compute reject_rate_pct
7) Keep docs honest: which parts are templates vs verified.

F) New examples
Add examples:
- /examples/btcminer_profiles_list.basil: prints PROFILES@()
- /examples/btcminer_profile_test.basil:
    - loads a profile by name
    - queries INFO@ and STATS@
    - prints normalized fields + raw payload keys

G) Generate a Next Steps document
Create docs/next-steps-obj-btcminer.md that includes:
- What’s done in v1
- Known gaps / tech debt (vendor endpoint variance, auth complexity)
- Proposed v1.1 milestones:
    1) Add Stratum worker/pool visibility (optional)
    2) Add vendor-specific adapters (antminer, whatsminer, braiins) beyond generic profiles
    3) Add better auth (digest, session renewal)
    4) Add a tiny local HTTP status server (optional)
    5) Add Prometheus exporter (optional)
- Testing plan:
    - how to run profile_test against a device
    - how to validate fields
- Safety and ethics note:
    - module is for monitoring/orchestration; no wallet custody; no malware behavior.

Acceptance:
- Generic-http-json adapter must work end-to-end with at least one “mock” profile and mock JSON fixtures in unit tests.
- Vendor profiles may be templates, but must be well-documented and usable with minimal edits.

DO NOT INCLUDE THIS FEATURE in --features obj-all, require explicit feature flag.