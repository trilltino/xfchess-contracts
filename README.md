# xfchess-contracts

Standalone extraction of the XFChess Solana program. Source of truth: [trilltino/XFChess](https://github.com/trilltino/XFChess), source commit `ca1eec71001392effe569da73e7cdc82db59c0d6`. `programs/`, `crates/nimzovich_engine`, `crates/xfchess-rules`, `crates/xfchess-adjudication`, `crates/xfchess-client-compat` and `clients/rust/instructions.rs` are byte-identical to that commit (XFChess keeps the chess crates under `crates/chess/` and the compat crate under `crates/solana/`).

Since the 2026-10-25 release the program has a fresh program ID, the legacy instructions are gone and every former `*_v2` instruction carries the plain name, the on-chain friends graph moved to the backend, Elo is integer fixed-point with variable K, and FIDE draw claims (`claim_fifty_move_draw`, `claim_repetition_draw`) were added. `PlayerProfile` and `Game` layouts are not compatible with accounts created by the old program.

Anchor/SPL 1.2.1, ER SDK 0.17.3, Solana Program 5.1.0, SDK 5.0.0 and Agave test/RPC 4.3.2. Program ID on devnet/localnet: `6idpQeH5YzXUT4sg8mG8dzxdK7ytmxfEyWkzUcNJJv4N` (the previous devnet program `8tevgspityTTG45KvvRtWV4GZ2kuGDBYWMXouFGquyDU` is retired and left untouched). Default features: `cranks`, `move-validation`, `devnet`. The opt-in `devnet-reset` feature adds `admin_close_account` and refuses to compile with `mainnet`.

The strict all-transitive-latest requirement remains blocked by upstream constraints and the noncompiling published bincode 3.0.0. Findings, advisories and verification limits are recorded in [the verification record](docs/security/verification-20261024.md). Mainnet placeholders intentionally fail production compilation. This repository is not an independent security sign-off.

| Path | Contents |
|---|---|
| programs/xfchess-game | Program source and SBF/host regressions |
| programs/xfchess-game-sbf | Deployable cdylib shim using the same source |
| crates/nimzovich_engine | Local no_std engine matching XFChess's pinned source; latest Shakmaty test API migrated here |
| crates/xfchess-rules | Host legality and PGN rules |
| crates/xfchess-adjudication | Allocation-free FIDE adjudication (clock, repetition, draw claims) shared by the program, engine and rules |
| crates/xfchess-client-compat | Current SDK/Agave transaction and account boundary |
| clients/rust/instructions.rs | Exact versioned Rust instruction builder used by client-layout fixtures |
| idl/xfchess_game.json | Regenerated IDL, 71 instructions (default features; `admin_close_account` is `devnet-reset` only) |
| docs/security | XFChess-source review evidence, dependency inventory and deployment provenance |

Build with cargo-build-sbf 4.5.0 and cached/downloaded platform-tools v1.58:

```sh
NO_DNA=1 cargo build-sbf --arch v0 --tools-version v1.58 --manifest-path programs/xfchess-game-sbf/Cargo.toml -- --locked
python3 docs/security/fetch_pass_fixture.py
cargo test -p xfchess-game -p xfchess-client-compat --locked -- --test-threads=1
cargo test -p nimzovich_engine -p xfchess-rules --locked
```

The pass fixture is fetched read-only from devnet and checked against its reviewed hash. Four network tests are explicitly ignored by default. The live ER test requires a separately started local L1/ER stack on fixed loopback ports, freshly generated in-memory signers and a localnet-feature artifact; it cannot exercise devnet. Do not run privileged ignored devnet tests without the necessary role authorization.

Original full-repository evidence under docs/security refers to XFChess paths and its exact lockfile/artifact. Extraction-specific evidence is recorded separately. The local engine path changes build provenance: an extraction rebuild is not automatically byte-identical to the devnet artifact. See [DEPLOYED.md](DEPLOYED.md).

No authority keypairs are included. License: AGPL-3.0-or-later.
