# CRYPTO -- Digital wallets for Basil -- thoughts

# What’s worth doing (practical + low-risk)

## Phase 0 — Utilities (no blockchain access needed)

* **Address & unit helpers** (BTC, ETH):

    * `IsValidAddress$(addr$)`, `ToChecksumAddress$(eth_addr$)`
    * `ToSats(n)`, `FromSats(n)`, `ToWei(n)`, `FromGwei(n)`
* **QR codes** for payment URIs: `MakePaymentQR$("bitcoin:…")` → PNG path.
* **Seed/Xpub tools (no private keys)**:

    * `MakeBip39Mnemonic$(words%)` (12/24), `Xpub$(mnemonic$, derivation_path$)`
    * `DeriveAddress$(xpub$, index%)` (watch-only)
* **Price lookups** (optional): via your HTTP mod to public APIs; cache & rate-limit. Read-only only.

## Phase 1 — Node-backed “do the thing” objects (still safe)

* **Bitcoin (watch-only + PSBT)** using `rust-bitcoin`:

    * `BTC_WATCH` (xpub): derive receive addresses, track UTXOs (via a public indexer or your own node), estimate fees.
    * `BTC_PSBT`: build unsigned transactions from inputs/outputs, **export PSBT**; import **signed PSBT** and broadcast.
      *No private keys in Basil; signing happens in a hardware wallet or external app.*
* **Ethereum (read/write without private keys)** using JSON-RPC:

    * `ETH_RPC`: `GetBalance(addr$)`, `GetNonce(addr$)`, `GasPrice()`, `Call$(to$, data_hex$)`, `SendRawTx$(signed_tx_hex$)`
      *User signs elsewhere (Ledger, MetaMask, a separate signer service).*
* **Lightning (optional later)**: `LND` or `CLN` client for **invoice creation & status** only (no node management).

## Phase 2 — Power features (opt-in, still key-safe)

* **Bitcoin Core RPC** (if the user runs a node): `ListUnspent`, `FundPsbt`, `SubmitPsbt`, `EstimateSmartFee`, `WatchAddress`.
* **ERC-20 helpers**: `TokenBalance(addr$, contract$)`, `BuildTransferData(to$, amount)` (still signed elsewhere).

# What to avoid (for Basil’s scope)

* Holding private keys or seed phrases in Basil memory.
* Integrating exchanges / KYC flows.
* Wallet UIs, seed backups, or anything custodial.
* Low-level cryptography re-implementation (use vetted crates).

# How it fits your mod system

**Crates / features**

* `basil-objects-crypto/` (you already planned for PGP):

    * add `obj-crypto-btc` (PSBT + address/units)
    * add `obj-crypto-eth` (JSON-RPC helpers)
    * add `obj-crypto-qr` (QR generation, also useful elsewhere)
* `basil-objects-net/` (you have HTTP/REST client planned): reuse for price feeds & Ethereum JSON-RPC.

**Objects (Basil surface)**

* `CRYPTO_BTC`

    * `IsValidAddress$()`, `ToSats()`, `FromSats()`
    * `NewXpubWatcher$(xpub$)` → `BTC_WATCH@`
    * `BuildPsbt$(inputs_json$, outputs_json$, fee_rate_sat_vb)` → psbt_base64$
    * `FinalizeAndBroadcast$(psbt_base64_signed$)` → txid$
* `BTC_WATCH`

    * `NextReceive$(path$, index%)` → address$
    * `Utxos$(address$|xpub$)` → JSON$
    * `EstimateFee$(target_blocks%)`
* `CRYPTO_ETH`

    * `IsValidAddress$()`, `ToWei()`, `FromGwei()`
    * `GetBalance$(addr$)`, `GetNonce$(addr$)`, `GasPrice$()`
    * `BuildErc20TransferData$(contract$, to$, amount_wei$)` → data_hex$
    * `SendRawTx$(signed_tx_hex$)` → txhash$
* `CRYPTO_QR`

    * `PaymentQRToFile$(uri$, out_path$)` → ok%

**Under the hood (Rust crates)**

* Bitcoin: `rust-bitcoin`, `bitcoin_hashes`, `miniscript` (for PSBT & descriptors).
* Ethereum: lightweight JSON-RPC via your existing HTTP/REST mod; hex utils (`alloy-primitives` or simple helpers).
* QR: `qrcode` + `image` crates.

# Developer experience in BASIC (tiny examples)

```basic
REM BTC PSBT flow (sign elsewhere)
DIM btc@ AS CRYPTO_BTC()
PRINT btc@.IsValidAddress$("bc1q...")

DIM psbt$ = btc@.BuildPsbt$("[{""txid"":""..."",""vout"":0}]", "{""bc1qdest…"": 10000}", 8.5)
PRINT "PSBT:", psbt$

REM user signs PSBT in hardware wallet → returns base64
PRINT "TXID:", btc@.FinalizeAndBroadcast$(signed_psbt$)
```

```basic
REM ETH read + raw send
DIM eth@ AS CRYPTO_ETH("https://mainnet.infura.io/v3/…")
PRINT "Balance (wei):", eth@.GetBalance$("0xabc…")
REM After signing elsewhere:
PRINT "Tx:", eth@.SendRawTx$("0x02f86b…")
```

```basic
REM Price (optional, via HTTP)
DIM http@ AS HTTP()
PRINT http@.Get$("https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd")
```

# Security & footguns (state it in docs)

* **No private keys** in Basil by default. If you ever add them, gate behind a distinct feature and strong warnings.
* **Be explicit about units** (sats/wei/gwei/ether); never guess.
* **Testnet support** (BTC testnet / ETH sepolia) via constructor arg or property, to keep users safe while experimenting.
* **Big integers/precision**: represent amounts as strings to avoid float issues.

# My recommendation

Start with **Phase 0 + BTC PSBT + ETH JSON-RPC (send-raw)**. It gives users real value (addresses, payments, balances, broadcasting) while keeping Basil out of custody and security headaches. If you want, I can draft a Junie prompt to scaffold **CRYPTO_BTC (PSBT)** + **CRYPTO_ETH (read/send-raw)** + **CRYPTO_QR** with examples and a short “Crypto README” for end users.
