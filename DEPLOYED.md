# Deployed devnet program

| Field | Verified value |
|---|---|
| Program | `8tevgspityTTG45KvvRtWV4GZ2kuGDBYWMXouFGquyDU` (devnet) |
| ProgramData | `DqgseskxrRYcgBpEks4jqh6eHbpg64SXxtGS1yhW9T7F` |
| Authority | `C1vn2MT7tZotZPjUJQDf9oo3dpZZ2tr7NxYLg8jTYgkw` (unchanged) |
| Finalized deployment slot | 515089691 |
| Transaction | [Upgrade](https://explorer.solana.com/tx/VrFEw8FvxaDsrKBhK77sUwc63AWpSpbk4KoYYFu9jqpUp7Q3aYP9wN9twwj2G4vXSriFdeHwNJXVo3Zw5dsqNku?cluster=devnet) |
| Source | [XFChess 133fbc7677c38c6395a48a97b714ee6aec6ce525](https://github.com/trilltino/XFChess/commit/133fbc7677c38c6395a48a97b714ee6aec6ce525) (branch `audit/contracts-hardening-20261025`) |
| Artifact | 1,156,240 bytes; SHA-256 `e6ab6731557ba9c45f6fce329a048f8edd69cfe0e1e9549f9766b0456b0a358e` |
| IDL | 82 instructions, old instruction data and Game/Profile layouts preserved |

The program source is copied from the verified XFChess release. The extraction uses a local engine path; its independently rebuilt artifact must be recorded separately and is not presumed byte-identical. The deployed artifact was produced in XFChess with cargo-build-sbf 4.5.0, platform-tools v1.58 (Rust 1.95), arch v0, locked dependencies and default `cranks,move-validation,devnet` features. Solana CLI 4.3.2 performed the upgrade with preflight and activated-feature verification enabled, without extending allocation or changing authority.

Upgraded on 2026-10-25 from the audit-hardening commit (see [execution record](docs/security/contracts-audit-execution-20261025.md)). It was built from a clean `git archive` of that commit with cargo-build-sbf 4.4.0, platform-tools v1.58, `--arch v0`, `--locked`; the same procedure on the previous release source reproduced the previous deployed hash byte for byte. Re-dumping ProgramData after the upgrade gave the hash above followed by zero padding; allocation (1,424,109 bytes) and upgrade authority are unchanged. No ABI change: the IDL and instruction snapshot are the same 82 instructions. The extraction build of this source is 1,156,760 bytes (`0d0096ce9af3464bb8463c01f418cdde7bbecd8ed3da7295cddf50325d12e941`), 520 bytes larger only because of embedded engine source paths, so it is not interchangeable release evidence.

Independent public-RPC verification checked the upgradeable loader, Program-to-ProgramData pointer, exact code hash and zero trailing padding. ProgramData allocation is 1,424,109 bytes including its 45-byte metadata header. See [release record](docs/security/release-20261025.md) and the public JSON evidence alongside it.

The on-chain engine source matches the pinned engine at `08e430699297f7d13f94fae32128fc0bafedf778`. This extraction additionally migrates its differential test calls to the latest Shakmaty API; those edits do not alter the engine runtime.

Wiped historical games without a verified recovery identity remain locked. Strict all-transitive-latest and the remaining audit scopes are not declared complete. Production placeholder guards remain enabled; mainnet deployment was not performed.
