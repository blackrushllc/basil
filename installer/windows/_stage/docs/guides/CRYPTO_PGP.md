# CRYPTO_PGP — PGP/GPG encryption, decryption, signing, verification

CRYPTO_PGP is a feature-gated Basil object that provides PGP/GPG operations using the Sequoia OpenPGP library. It uses a string-centric API: you pass ASCII‑armored keys and messages, and you receive ASCII‑armored outputs or write to files.

Enable it with Cargo features:

- Single feature:  cargo build --features obj-crypto-pgp
- Umbrella (for future crypto objects): cargo build --features obj-crypto
- All objects: cargo build --features obj-all

When the feature is disabled, the object is not registered. When enabled, DESCRIBE will show its methods.

## Constructors

- DIM pgp@ AS CRYPTO_PGP()

No arguments. A stateless helper object.

## Methods

- EncryptArmored$(public_key_armored$, plaintext$) → cipher_armored$
  Encrypts a string to the recipient’s public key. Returns an ASCII‑armored PGP message.

- DecryptArmored$(private_key_armored$, passphrase$?, cipher_armored$) → plaintext$
  Decrypts an ASCII‑armored PGP message using the private key (optionally passphrase‑protected).

- SignArmored$(private_key_armored$, passphrase$?, message$) → signature_armored$
  Produces a detached, ASCII‑armored signature for the given message bytes.

- Verify$(public_key_armored$, message$, signature_armored$) → ok%
  Verifies a detached signature against a public key. Returns 1 if valid else 0.

- EncryptFile$(public_key_armored$, in_path$, out_path$) → ok%
  Reads input file, encrypts to recipient key, writes armored message to out_path.

- DecryptFile$(private_key_armored$, passphrase$?, in_path$, out_path$) → ok%
  Reads armored message from in_path, decrypts, writes plaintext bytes to out_path.

- SignFile$(private_key_armored$, passphrase$?, in_path$, sig_out_path$) → ok%
  Produces a detached signature and writes it as an ASCII‑armored file.

- VerifyFile$(public_key_armored$, in_path$, sig_path$) → ok%
  Verifies a detached signature file against in_path. Returns 1 if valid else 0.

- ReadFileText$(path$) → text$
  Convenience helper to read small text files (armored keys/messages).

- WriteFileText$(path$, text$) → ok%
  Convenience helper to write text files (armored messages/signatures).

Notes:
- Phase 1 supports armor only and detached signatures (no clearsign). Encryption to a single recipient is supported.
- Compression and cipher choices rely on library defaults.

## Key generation quickstart (GnuPG)

1) Generate a key (never‑expiring for demo):

    gpg --quick-generate-key "Your Name <you@example.com>" default default never

2) Export public key:

    gpg --export --armor you@example.com > pub.asc

3) Export secret key:

    gpg --export-secret-keys --armor you@example.com > sec.asc

If your secret key is passphrase‑protected, pass that value as passphrase$ in the Decrypt/Sign methods (empty string if none).

## Examples

Place the following in your project root (or working dir): pub.asc, sec.asc, input.txt.

- examples/pgp_encrypt_decrypt.basil

  REM Encrypt & decrypt strings with armored keys

  DIM pub$ = READFILE$("pub.asc")
  DIM sec$ = READFILE$("sec.asc")
  LET pass$ = ""          REM or your passphrase

  DIM pgp@ AS CRYPTO_PGP()
  LET msg$ = "Hello from Basil PGP!"

  LET cipher$ = pgp@.EncryptArmored$(pub$, msg$)
  PRINTLN "CIPHER:\n", cipher$

  LET plain$ = pgp@.DecryptArmored$(sec$, pass$, cipher$)
  PRINTLN "PLAIN:\n", plain$

- examples/pgp_sign_verify.basil

  REM Detached sign & verify

  LET pub$ = READFILE$("pub.asc")
  LET sec$ = READFILE$("sec.asc")
  LET pass$ = ""

  DIM pgp@ AS CRYPTO_PGP()
  LET data$ = "Important message"

  LET sig$ = pgp@.SignArmored$(sec$, pass$, data$)
  PRINTLN "SIG:\n", sig$

  PRINTLN "VERIFY OK? ", pgp@.Verify$(pub$, data$, sig$)

- examples/pgp_file_ops.basil

  REM File encrypt/decrypt/sign/verify

  LET pub$ = READFILE$("pub.asc")
  LET sec$ = READFILE$("sec.asc")
  LET pass$ = ""

  DIM pgp@ AS CRYPTO_PGP()

  PRINTLN "Encrypt file: ", pgp@.EncryptFile$(pub$, "input.txt", "input.txt.asc")
  PRINTLN "Decrypt file: ", pgp@.DecryptFile$(sec$, pass$, "input.txt.asc", "input.out.txt")
  PRINTLN "Sign file: ", pgp@.SignFile$(sec$, pass$, "input.txt", "input.txt.sig")
  PRINTLN "Verify file: ", pgp@.VerifyFile$(pub$, "input.txt", "input.txt.sig")

## Usage guidance

- Detached signatures: preferred when you want to keep data unchanged and store the signature separately. Use Verify/VerifyFile with your public key.
- Encrypt‑then‑sign: You may encrypt data to someone and separately sign the original or the ciphertext. Phase 1 provides detached signatures; clearsign may arrive later.

## Security notes

- Never commit private keys (sec.asc) to your repository. Prefer file paths and environment variables for sensitive material.
- Passphrases are handled in memory only for unlocking; consider secure prompts or env vars in higher layers.
- This module does not use or modify any OS keyring; only the provided armored keys are used.

## Exceptions and error messages

Errors are mapped to short, readable messages. Examples:
- PGP.Encrypt: KeyParseError — The public key couldn’t be parsed or had no usable encryption subkey.
- PGP.Decrypt: KeyParseError — Private key parse failed.
- PGP.Decrypt: BadPassphrase — Wrong passphrase (reported as a decryption failure).
- PGP.Verify: SignatureInvalid — Signature does not verify against the provided public key.

No secret material or passphrase values are logged in error messages.
