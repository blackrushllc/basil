# Junie Pro task: Basil **CRYPTO_PGP** (PGP/GPG encryption, decryption, signing, verification)

## Context

Basil supports feature-gated objects (e.g., obj-ai, obj-aws). We want a **PGP/GPG** crypto module that keeps a **Basil-native, string-centric** surface: pass in ASCII-armored keys or file paths, return armored text or write files. No git actions; no tests required in this pass. **You must include runnable examples and a user doc.**

## Crate & features

Create a new crate: `basil-objects-crypto/` with features:

* `obj-crypto-pgp` (enables CRYPTO_PGP)
* optional umbrella `obj-crypto = ["obj-crypto-pgp"]`

Wire via workspace deps (forward slashes) and register the object with the global registry only when the feature is enabled.

## Dependencies (Rust)

Use **Sequoia OpenPGP** (mature Rust implementation):

* `sequoia-openpgp` (core)
* `sequoia-openpgp`’s helpers for armor/parse/verify
  No async/tokio needed (pure CPU/buffer). Keep binary size contained via feature gating.

## Object: `CRYPTO_PGP`

Goal: stateless helpers; take keys as **armored strings** or **file paths** (caller’s choice). No OS keyring.

### Constructors

* `DIM pgp@ AS CRYPTO_PGP()` — no args; pure helpers object.

### Methods (Phase 1)

Armor-in/armor-out string helpers:

* `EncryptArmored$(public_key_armored$, plaintext$)` → `cipher_armored$`
  (Opaque PGP message to a single recipient; support multiple recipients via comma-joined keys if trivial, otherwise Phase 2.)
* `DecryptArmored$(private_key_armored$, passphrase$?, cipher_armored$)` → `plaintext$`
* `SignArmored$(private_key_armored$, passphrase$?, message$)` → `signature_armored$`
  (Detached signature; message is not armored; returns ASCII-armored signature.)
* `Verify$(public_key_armored$, message$, signature_armored$)` → `ok%` (1 if valid else 0)

File helpers (stream to/from disk; avoid loading huge files wholly in memory):

* `EncryptFile$(public_key_armored$, in_path$, out_path$)` → `ok%`
* `DecryptFile$(private_key_armored$, passphrase$?, in_path$, out_path$)` → `ok%`
* `SignFile$(private_key_armored$, passphrase$?, in_path$, sig_out_path$)` → `ok%` (detached)
* `VerifyFile$(public_key_armored$, in_path$, sig_path$)` → `ok%`

Key I/O convenience (optional but recommended):

* `ReadFileText$(path$)` → file_text$   (small helper to read keys/messages)
* `WriteFileText$(path$, text$)` → ok%   (for writing armored outputs)

### Notes & constraints

* **Armor only** for keys/packets in Phase 1. (Binary packets are out of scope; armor is most copy-paste friendly.)
* **Detached signatures** (not clearsign) in Phase 1; clearsign can be Phase 2.
* **Single recipient** encryption minimum; multiple recipients can be a follow-up (API: pass keys joined with `\n\n` or array—string array support exists; choose easiest path).
* **Compression**: use Sequoia’s default (or none); not user-tunable in Phase 1.
* **Ciphers/algorithms**: accept library defaults; no knobs needed in Phase 1.

### Errors → Basil exceptions

Map all failures to readable Basil exceptions:

* Examples: `PGP.Encrypt: KeyParseError`, `PGP.Decrypt: BadPassphrase`, `PGP.Verify: SignatureInvalid`
* Do **not** log secret material; redact passphrases.

## Docs (must include)

Create `docs/guides/PGP.md` explaining for end users:

* What CRYPTO_PGP provides (methods table with short descriptions).
* **Key generation** quickstart with GnuPG:

  * `gpg --quick-generate-key "Your Name <you@example.com>" default default never`
  * Export public: `gpg --export --armor you@example.com > pub.asc`
  * Export secret: `gpg --export-secret-keys --armor you@example.com > sec.asc`
  * If passphrase protected, supply it in `passphrase$`.
* Basic usage patterns and guidance (detached signatures vs encrypt-then-sign).
* Security notes:

  * Keep private keys out of your repo; prefer file paths over embedding secrets.
  * Passphrases live in memory briefly; recommend env vars or secure prompts (future).
* Feature flags and build examples:

  * `cargo build --features obj-crypto-pgp`

## Examples (must include; runnable)

Place under `/examples/`:

1. **`pgp_encrypt_decrypt.basil`**

```
REM Encrypt & decrypt strings with armored keys

DIM pub$ = READFILE$("pub.asc")       REM provide these files locally
DIM sec$ = READFILE$("sec.asc")
LET pass$ = ""                         REM or "your-passphrase" if key is protected

DIM pgp@ AS CRYPTO_PGP()
LET msg$ = "Hello from Basil PGP!"

LET cipher$ = pgp@.EncryptArmored$(pub$, msg$)
PRINTLN "CIPHER:\n", cipher$

LET plain$ = pgp@.DecryptArmored$(sec$, pass$, cipher$)
PRINTLN "PLAIN:\n", plain$
```

2. **`pgp_sign_verify.basil`**

```
REM Detached sign & verify

LET pub$ = READFILE$("pub.asc")
LET sec$ = READFILE$("sec.asc")
LET pass$ = ""

DIM pgp@ AS CRYPTO_PGP()
LET data$ = "Important message"

LET sig$ = pgp@.SignArmored$(sec$, pass$, data$)
PRINTLN "SIG:\n", sig$

PRINTLN "VERIFY OK? ", pgp@.Verify$(pub$, data$, sig$)
```

3. **`pgp_file_ops.basil`**

```
REM File encrypt/decrypt/sign/verify

LET pub$ = READFILE$("pub.asc")
LET sec$ = READFILE$("sec.asc")
LET pass$ = ""

DIM pgp@ AS CRYPTO_PGP()

PRINTLN "Encrypt file: ", pgp@.EncryptFile$(pub$, "input.txt", "input.txt.asc")
PRINTLN "Decrypt file: ", pgp@.DecryptFile$(sec$, pass$, "input.txt.asc", "input.out.txt")
PRINTLN "Sign file: ", pgp@.SignFile$(sec$, pass$, "input.txt", "input.txt.sig")
PRINTLN "Verify file: ", pgp@.VerifyFile$(pub$, "input.txt", "input.txt.sig")
```

*(These examples assume you have `pub.asc`, `sec.asc`, and `input.txt` in the working directory. The doc should mention how to produce them.)*

## Implementation notes (guidance)

* Parsing keys: use Sequoia’s `Cert`/`Packet` reading from armor; handle common errors with friendly messages.
* Decryption: unlock secret key (passphrase optional); fail cleanly on wrong passphrase.
* Signing: create **detached** signatures (binary mode) for strings and files.
* Verification: return `1` for valid, `0` for invalid; include failure reason in exception when invalid.
* File ops: use streaming readers/writers to avoid large allocations.
* Keep API **pure** (no global state), thread-safe enough for future parallel calls.

## Acceptance checklist

* [ ] Workspace builds with `--features obj-crypto-pgp` and also when the feature is off.
* [ ] `--features obj-all` includes `obj-crypto-pgp` !!!
* [ ] `CRYPTO_PGP` registers only when the feature is enabled; `DESCRIBE pgp@` shows methods.
* [ ] `/examples/pgp_encrypt_decrypt.basil`, `/examples/pgp_sign_verify.basil`, `/examples/pgp_file_ops.basil` exist and run with local test keys.
* [ ] `docs/integrations/crypto/pgp.md` exists and covers key generation, usage, feature flags, and exception handling notes.
* [ ] Errors are readable and never leak private material.

