# Basil Library Objects Reference

This document provides a concise function and object reference for all feature-backed library objects available in Basil, including the BMX objects for completeness. Each section lists the relevant keywords, a description, and a small runnable example.

Note: Many of these capabilities are gated behind Cargo features. See the “Feature flags” note in each section for how to enable them when running examples.

Last updated: 2025-10-08 01:24 (local)

---

## BASE64

- Feature flag: obj-base64
- Purpose: Base64 encode/decode text.

Keywords
- Functions (built-ins):
  - BASE64_ENCODE$(text$) -> string
  - BASE64_DECODE$(text$) -> string
- Object (alternative usage):
  - Type: BASE64
  - Methods: Encode$(text$) -> string, Decode$(text$) -> string

Description
- Provides Base64 encode/decode both as global functions and via a small utility object. Decoding errors produce a runtime error (invalid Base64 or invalid UTF-8).

Example (functions)
```
PRINTLN "Base64 demo"
LET a$ = "Hello, World!"
LET encoded$ = BASE64_ENCODE$(a$)
LET decoded$ = BASE64_DECODE$(encoded$)
PRINTLN "Original: ", a$
PRINTLN "Encoded : ", encoded$
PRINTLN "Decoded : ", decoded$
```

Example (object)
```
DIM b@ AS BASE64()
PRINTLN b@.Encode$("Hello")
PRINTLN b@.Decode$(b@.Encode$("ABC"))
```

How to run (Windows PowerShell)
- cargo run -q -p basilc --features obj-base64 -- run examples\base64.basil

---

## CURL (HTTP helpers)

- Feature flag: obj-curl
- Purpose: Simple HTTP GET/POST helpers.

Keywords
- HTTP_GET$(url$) -> string (response body)
- HTTP_POST$(url$, body$[, content_type$]) -> string (response body)

Description
- Performs HTTP requests using a simple interface. Non-2xx HTTP responses raise a runtime error showing status code and text.
- Default content type for POST is "text/plain; charset=utf-8". Pass an explicit content type for JSON uploads (e.g., "application/json").

Example
```
PRINTLN "CURL demo"
LET url$ = "https://yobasic.com/basil/hello.basil"
PRINTLN "GET ", url$
LET body$ = HTTP_GET$(url$)
PRINTLN "Response length: ", LEN(body$)
PRINTLN "Preview (first 120 chars):"
PRINTLN LEFT$(body$, 120)

LET post_url$ = "https://httpbin.org/post"
LET json$ = "{\"hello\": \"world\", \"number\": 123}"
LET resp$ = HTTP_POST$(post_url$, json$, "application/json")
PRINTLN "POST to httpbin length: ", LEN(resp$)
```

How to run (Windows PowerShell)
- cargo run -q -p basilc --features obj-curl -- run examples\curl.basil

---

## JSON helpers

- Feature flag: obj-json
- Purpose: Parse/normalize JSON and stringify values.

Keywords
- JSON_PARSE$(text$) -> string (canonical JSON)
- JSON_STRINGIFY$(value) -> string (JSON)

Description
- JSON_PARSE$ parses text into JSON and re-serializes it in canonical/minified form. Errors if input is not valid JSON.
- JSON_STRINGIFY$ accepts either:
  - A JSON string: it parses and normalizes it.
  - Any other string: it wraps it as a JSON string (adds quotes and escapes).
  - Arrays and other values: they are converted to JSON where possible.

Example
```
PRINTLN "JSON demo"
LET raw$ = "{\"name\":\"Erik\",\"age\":60,\"likes\":[\"Biking\",\"Hiking\"]}"
LET canon$ = JSON_PARSE$(raw$)
PRINTLN "Normalized: ", canon$

LET wrapped$ = JSON_STRINGIFY$("Hello, World!")
PRINTLN "Wrapped: ", wrapped$

LET already$ = JSON_STRINGIFY$(raw$)
PRINTLN "Already JSON -> normalized: ", already$

DIM A$(3)
LET A$(0) = "a"
LET A$(1) = "b"
LET A$(2) = "c"
LET json$ = JSON_STRINGIFY$(A$)
PRINTLN "Array: ", json$
```

How to run (Windows PowerShell)
- cargo run -q -p basilc --features obj-json -- run examples\json_demo.basil

---

## CSV helpers

- Feature flag: obj-csv
- Purpose: Convert between CSV and JSON-array-of-objects strings.

Keywords
- CSV_PARSE$(csv_text$) -> string (JSON array of objects)
- CSV_WRITE$(rows_json$) -> string (CSV)

Description
- CSV_PARSE$ expects a header row and yields a JSON array where each row is an object with header names as keys and cell values as strings.
- CSV_WRITE$ expects a JSON array of objects; it infers headers from the first object and any new keys seen later, then emits a CSV.

Example
```
PRINTLN "CSV demo"
LET csv$ = "name,age\nErik,59\nJunie,?\n"
LET rows$ = CSV_PARSE$(csv$)
PRINTLN "As JSON: ", rows$
LET out$ = CSV_WRITE$(rows$)
PRINTLN "Roundtrip CSV:\n", out$
```

How to run (Windows PowerShell)
- cargo run -q -p basilc --features obj-csv -- run examples\csv_demo.basil

---

## ZIP helpers

- Feature flag: obj-zip
- Purpose: Create, list, and extract ZIP archives.

Keywords
- ZIP_EXTRACT_ALL(zip_path$, dest_dir$) -> string (empty string on success)
- ZIP_COMPRESS_FILE(src_path$, zip_path$[, entry_name$]) -> string (empty string on success)
- ZIP_COMPRESS_DIR(src_dir$, zip_path$) -> string (empty string on success)
- ZIP_LIST$(zip_path$) -> string (newline-separated entries)

Description
- ZIP_EXTRACT_ALL extracts every entry into the destination directory, creating folders as needed.
- ZIP_COMPRESS_FILE compresses one file into a new zip (with optional entry name).
- ZIP_COMPRESS_DIR zips a directory recursively, preserving folder structure.
- ZIP_LIST$ returns a newline-separated listing of the archive’s entries.

Example
```
PRINTLN "ZIP demo"
ZIP_COMPRESS_FILE("README.md", "onefile.zip", "README.md")
PRINTLN "Created onefile.zip"

ZIP_COMPRESS_DIR("examples", "examples.zip")
PRINTLN "Created examples.zip"

LET list$ = ZIP_LIST$("examples.zip")
PRINTLN "Entries in examples.zip:"
PRINTLN list$

ZIP_EXTRACT_ALL("examples.zip", "unzipped")
PRINTLN "Extracted to ./unzipped"
```

How to run (Windows PowerShell)
- cargo run -q -p basilc --features obj-zip -- run examples\zip_demo.basil

---

## BMX_RIDER object

- Feature flag: obj-bmx-rider (included by obj-bmx)
- Purpose: Represents an individual BMX rider.

Keywords
- Type: BMX_RIDER(name$, age%, skillLevel$, wins%, losses%)
- Properties (read/write): Name$, Age%, SkillLevel$, Wins%, Losses%
- Methods:
  - Describe$() -> string
  - TotalRaces%() -> integer
  - WinRate() -> float (0.0..1.0)
  - Tags$() -> string[]
  - Info$() -> string

Description
- A simple object modeling a BMX rider profile with convenience methods for statistics and display.

Example
```
#USE BMX_RIDER
DIM r@ AS BMX_RIDER("Alice", 17, "Expert", 12, 3)
PRINTLN r@.Describe$()
PRINTLN "Total races:", r@.TotalRaces%()
PRINTLN "Win rate:", r@.WinRate()*100, "%"
```

---

## BMX_TEAM object

- Feature flag: obj-bmx-team (included by obj-bmx)
- Purpose: Represents a team aggregating BMX riders.

Keywords
- Type: BMX_TEAM(name$, establishedYear%[, flags%])
- Constants: PRO = 1, NOT_PRO = 0
- Properties (read/write): Name$, EstablishedYear%, TeamWins%, TeamLosses%
- Methods:
  - AddRider(rider@) -> void
  - RemoveRider(name$) -> integer (1 if removed, 0 if not found)
  - WinPct() -> float (0.0..1.0)
  - TopRider() -> BMX_RIDER
  - BottomRider() -> BMX_RIDER
  - RiderNames$() -> string[]
  - RiderDescriptions$() -> string[]
  - Info$() -> string

Description
- Manage a roster of riders; query top/bottom performers by win rate with tie-breakers; track team stats; and mark a team as PRO via flags.

Example
```
#USE BMX_RIDER, BMX_TEAM

DIM r1@ AS BMX_RIDER("Alice", 17, "Expert", 12, 3)
DIM r2@ AS BMX_RIDER("Bob",   21, "Intermediate", 5, 10)
DIM r3@ AS BMX_RIDER("Carol", 19, "Pro", 30, 4)

DIM t@ AS BMX_TEAM("Rocket Foxes", 2015, PRO)
t@.AddRider(r1@)
t@.AddRider(r2@)
t@.AddRider(r3@)

PRINTLN t@.Info$()
PRINTLN "Top rider:", t@.TopRider().Describe$()
PRINTLN "All rider names:"
DIM names$@ = t@.RiderNames$()
FOR i = 0 TO UBOUND(names$@)
  PRINTLN names$@(i)
NEXT
```

How to run (Windows PowerShell)
- cargo run -q -p basilc --features obj-bmx -- run examples\objects.basil

---

## Feature summary

- obj-base64: BASE64_ENCODE$, BASE64_DECODE$, BASE64 object
- obj-curl: HTTP_GET$, HTTP_POST$
- obj-json: JSON_PARSE$, JSON_STRINGIFY$
- obj-csv: CSV_PARSE$, CSV_WRITE$
- obj-zip: ZIP_EXTRACT_ALL, ZIP_COMPRESS_FILE, ZIP_COMPRESS_DIR, ZIP_LIST$
- obj-bmx: BMX_RIDER, BMX_TEAM (and BMX constants PRO/NOT_PRO)

For a convenient "everything" build, there is often an aggregate feature (e.g., obj-all) in Cargo; check Cargo.toml for availability.
