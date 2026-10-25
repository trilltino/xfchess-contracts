# xfchess-contracts

Standalone extraction of the XFChess Solana program. Source of truth: [trilltino/XFChess](https://github.com/trilltino/XFChess), release source commit `fb168600806aca161ff423610bc9a6457476b494` ([PR #52](https://github.com/trilltino/XFChess/pull/52)). The extraction includes the approved recovery identity schema, versioned account/client transition, security regressions and current direct dependencies.

Anchor/SPL 1.2.1, ER SDK 0.17.3, Solana Program 5.1.0, SDK 5.0.0 and Agave test/RPC 4.3.2. Program ID on devnet/localnet: `8tevgspityTTG45KvvRtWV4GZ2kuGDBYWMXouFGquyDU`. Default features: `cranks`, `move-validation`, `devnet`.

The strict all-transitive-latest requirement remains blocked by upstream constraints and the noncompiling published bincode 3.0.0. Findings, advisories and verification limits are recorded in [SECURITY_FINDINGS.md](SECURITY_FINDINGS.md). Mainnet placeholders intentionally fail production compilation. This repository is not an independent security sign-off.

| Path | Contents |
|---|---|
| programs/xfchess-game | Program source and SBF/host regressions |
| programs/xfchess-game-sbf | Deployable cdylib shim using the same source |
| crates/nimzovich_engine | Local no_std engine matching XFChess's pinned source; latest Shakmaty test API migrated here |
| crates/xfchess-rules | Host legality and PGN rules |
| crates/xfchess-client-compat | Current SDK/Agave transaction and account boundary |
| clients/rust/instructions.rs | Exact versioned Rust instruction builder used by client-layout fixtures |
| idl/xfchess_game.json | Regenerated safety-linted IDL, 82 instructions |
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
