**Prompt for Junie (manual CSV compilation, no tool):**

Goal
Create curated CSVs (no generator) for keyword analysis.

Outputs (commit as plain files):

* `tools/keyword_catalog/out/basil_keywords_catalog.csv`
* `tools/keyword_catalog/out/keyword_equivalents.csv`
* `tools/keyword_catalog/out/language_coverage.csv` (simple rollup you compute by hand or with spreadsheet formulas)

CSV #1 schema — `basil_keywords_catalog.csv`
Columns (header row required):
`namespace,language,keyword,kind,availability,description,aliases,source,notes`

Rules

1. **Basil CORE**: Extract every user-facing keyword & statement (e.g., PRINT, INPUT, IF, ELSEIF / ELSE IF, SELECT CASE / END SELECT, FOR / NEXT, FOR EACH, WHILE / END WHILE, TYPE / END TYPE, BEGIN / END, and the modern `{}` note as a *note*, not a keyword).

    * Include multi-word forms as a single `keyword` string with a space (e.g., `SELECT CASE`, `END TYPE`).
    * `namespace= "Basil"`, `language="Basil"`, `availability="core"`.
    * `kind`: pick the closest (statement, function, operator, type, flow, directive, io, module-proc, module-func).
    * `source`: point to the doc page or code location you used.
2. **Basil MODS**: For each feature object (obj-zip, obj-json, obj-sqlite, obj-ai, obj-term, obj-aws, etc.), list every exported procedure/function **by the Basil name developers call**.

    * `namespace="Basil-Mod"`, `language="Basil"`, `availability="mod:<modname>"`.
3. **Descriptions**: One line each (≤120 chars). Prefer docs wording; otherwise synthesize concise text.
4. **Aliases**: Use `;`-separated list for true alternates (e.g., `ELSEIF;ELSE IF`).
5. **Other Languages**: Add rows for common keywords from: Python, PHP, JS/TS, Java, C, C++, C#, Go, Rust, Swift, Kotlin, Ruby, Perl, Bash, SQL, Lua, Haskell, R, **and** classic BASICs (GW-BASIC, Bywater BASIC, QBasic/QuickBASIC, VB6).

    * `namespace=<Language>`, `language=<Language>`, `availability="n/a"`.
    * Keep descriptions short and neutral.
6. Sorting: sort rows by `language, keyword`.

CSV #2 — `keyword_equivalents.csv`
Header: `concept,language,form,kind,notes`
Populate a curated set of concepts (e.g., `elseif`, `function-declare`, `print`, `switch`, `continue`, `break`, `return`, `for-each`, `try-catch`, `finally`, `throw`, `import`, `class`, `struct`, `type`, `let/var/const`, `lambda`, `match/case`). For each concept, add the form per language (e.g., `ELSEIF`, `else if`, `elif`).

CSV #3 — `language_coverage.csv`
Header: `language,total_keywords,flow_count,io_count,type_count,operator_count,module_count,unique_only_here_count`
Compute quick counts by scanning your compiled CSV (spreadsheet formulas OK). “unique_only_here” = keywords not appearing in any other language.

Quality/consistency checklist

* Use **UPPERCASE** for Basil keywords (case-insensitive in practice; pick one canonical form).
* Include both `ELSEIF` and `ELSE IF` as separate Basil rows (alias link them).
* Treat `{}` as a **note** about modern blocks; don’t list `{` as a keyword.
* Keep BASIC multi-word enders (`END SELECT`, `END TYPE`, `END WHILE`, `NEXT`) as distinct rows.
* For mods, prefer dotted names only if that’s how users actually call them (e.g., `JSON.PARSE`).
* Mind CSV quoting: quote any field containing comma/quote/newline.

Delivery

* Place the three CSVs under `tools/keyword_catalog/out/`.
* Add a short `docs/KEYWORDS_CATALOG.md` explaining columns and how to update manually.

---

If you later find this getting stale, the lightest upgrade is to have Junie **keep the CSV but also paste the regexes/paths she used** at the bottom of `KEYWORDS_CATALOG.md`. That way, even without a generator, you have a repeatable manual process.

If you want, I can also whip up a tiny Basil example that reads the CSV and prints top gaps (but you didn’t require examples for this one).
