# xfchess-contracts

Standalone, self-contained extraction of the **XFChess Solana program** (`xfchess-game`)
for security / static analysis. Extracted 2026-10-09 from the working tree of
[trilltino/XFChess](https://github.com/trilltino/XFChess) at `b34518000` **plus uncommitted
changes** (notably `tournament_ix/lifecycle/cancel.rs` and `tests/tournament_cancel_tests.rs`).
The main repo remains the source of truth; this copy does not sync automatically.

- **Framework:** Anchor 1.1.2, Solana 3.0, MagicBlock `ephemeral-rollups-sdk` 0.16.2
- **Program ID (localnet + devnet):** `8tevgspityTTG45KvvRtWV4GZ2kuGDBYWMXouFGquyDU`
- **Default features:** `cranks`, `move-validation`, `devnet` (audit/mainnet target: `production`)

## Layout

| Path | Contents |
|------|----------|
| `programs/xfchess-game/src/` | The program — entry point `lib.rs`, see its `README.md` for the module map |
| `programs/xfchess-game/tests/` | solana-program-test suites (load `target/deploy/xfchess_game.so`) |
| `crates/nimzovich_engine/` | `no_std` chess move generation compiled **into the program** via `move-validation` (`src/on_chain*.rs` are the on-chain paths). Vendored from XFChess commit `bfaa6a133^`; upstream now lives in a separate repo |
| `idl/xfchess_game.json` | IDL as used by the web frontend (may lag the source — regenerate with `anchor idl build` if exact) |
| `docs/` | Threat model, audit tracking, ADRs, architecture, settlement/ER runbooks |
| `specs/` | TLA+ model of Solana finality assumptions |
| `XFChess_Smart_Contract_Audit_Spec.pdf` | Audit specification |
| `MAGICBLOCK.md`, `SECURITY.md` | Ephemeral Rollup lifecycle, disclosure policy |

## Build & test

```bash
cargo build-sbf --manifest-path programs/xfchess-game/Cargo.toml   # -> target/deploy/xfchess_game.so
cargo test -p xfchess-game
```

On Windows, test builds need a prebuilt OpenSSL (`solana-program-test` pulls in `openssl-sys`):

```powershell
$env:OPENSSL_NO_VENDOR="1"; $env:OPENSSL_DIR="C:\Program Files\OpenSSL-Win64"
$env:OPENSSL_LIB_DIR="C:\Program Files\OpenSSL-Win64\lib\VC\x64\MD"
$env:OPENSSL_INCLUDE_DIR="C:\Program Files\OpenSSL-Win64\include"
```

## Not included

- **Private keypairs.** `treasury_tests`, `er_recovery_tests`, `link_external_elo_tests` and
  `tournament_registration_e2e_tests` read devnet authority keys from `keys/*.json`, which are
  deliberately not copied. The authority **pubkeys** are hard-coded in
  `programs/xfchess-game/src/constants.rs`; `keys/KEYS_README.md` maps each to its role.
- Off-chain components (backend transaction builders, game client, web) — they live in the main repo.

License: AGPL-3.0-or-later (see `LICENSE`).
