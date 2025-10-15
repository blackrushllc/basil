# PGP/GPG with CRYPTO_PGP

This is the end‑user guide for the CRYPTO_PGP object. For the full integration document, see docs/integrations/crypto/pgp.md.

Quick start:

- Build with: cargo build --features obj-crypto-pgp
- Generate keys with GnuPG:
  - gpg --quick-generate-key "Your Name <you@example.com>" default default never
  - gpg --export --armor you@example.com > pub.asc
  - gpg --export-secret-keys --armor you@example.com > sec.asc

See examples in /examples:
- pgp_encrypt_decrypt.basil
- pgp_sign_verify.basil
- pgp_file_ops.basil

Security tips:
- Don’t commit private keys; prefer file paths and environment variables for secrets.
- Passphrases are only used to unlock keys in memory briefly.
