# Deployed devnet program

| Field | Verified value |
|---|---|
| Program | `8tevgspityTTG45KvvRtWV4GZ2kuGDBYWMXouFGquyDU` (devnet) |
| ProgramData | `DqgseskxrRYcgBpEks4jqh6eHbpg64SXxtGS1yhW9T7F` |
| Authority | `C1vn2MT7tZotZPjUJQDf9oo3dpZZ2tr7NxYLg8jTYgkw` (unchanged) |
| Finalized deployment slot | 514786564 |
| Transaction | [Upgrade](https://explorer.solana.com/tx/3u4N77sN31qvdzx8PFHuRuDp1LGyMGKw7wQc62qmc4iXoCkhJsLq5j9nShjeQju8ZrVVnmudum1APmjyDL7u4FSj?cluster=devnet) |
| Source | [XFChess fb168600806aca161ff423610bc9a6457476b494](https://github.com/trilltino/XFChess/commit/fb168600806aca161ff423610bc9a6457476b494) |
| Artifact | 1,152,400 bytes; SHA-256 `f60ce59fc4ac7cd02a83e520c54b75ea9d44c6127a986fa8efb01b3e431f74fe` |
| IDL | 82 instructions, old instruction data and Game/Profile layouts preserved |

The program source is copied from the verified XFChess release. The extraction uses a local engine path; its independently rebuilt artifact must be recorded separately and is not presumed byte-identical. The deployed artifact was produced in XFChess with cargo-build-sbf 4.5.0, platform-tools v1.58 (Rust 1.95), arch v0, locked dependencies and default `cranks,move-validation,devnet` features. Solana CLI 4.3.2 performed the upgrade with preflight and activated-feature verification enabled, without extending allocation or changing authority.

Independent public-RPC verification checked the upgradeable loader, Program-to-ProgramData pointer, exact code hash and zero trailing padding. ProgramData allocation is 1,424,109 bytes including its 45-byte metadata header. See [release record](docs/security/release-20261025.md) and the public JSON evidence alongside it.

The on-chain engine source matches the pinned engine at `08e430699297f7d13f94fae32128fc0bafedf778`. This extraction additionally migrates its differential test calls to the latest Shakmaty API; those edits do not alter the engine runtime.

Wiped historical games without a verified recovery identity remain locked. Strict all-transitive-latest and the remaining audit scopes are not declared complete. Production placeholder guards remain enabled; mainnet deployment was not performed.
