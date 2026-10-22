# Deployed program

| | |
|---|---|
| Program ID | `8tevgspityTTG45KvvRtWV4GZ2kuGDBYWMXouFGquyDU` (Solana devnet) |
| Upgrade authority | `C1vn2MT7tZotZPjUJQDf9oo3dpZZ2tr7NxYLg8jTYgkw` |
| Deployed at slot | 513888640 (upgrade tx `4WQeVRWGdNnz4TeEyLFBUv4agwFawCbtiZkuocCGgAtNrXr9WMXzsNHrV3ETntFzL6bawQCyoCb4UUi4nmipwa53`) |
| Source | [trilltino/XFChess](https://github.com/trilltino/XFChess) commit `70a25c5c4e2d2fef18bb30f54f4260a74e27fadb`, `programs/xfchess-game` |
| Deployed binary | 1,094,544 bytes, SHA-256 `f9a6d1c3f417c33e602a036fccdd8a01c970d092978319dd401c3192bc7f70c3` |
| Instructions | 73 (every `sha256("global:<name>")[..8]` dispatch discriminator is present in the binary) |

`programs/xfchess-game` in this repository is byte-identical to that commit. ProgramData on devnet is 1,424,064
bytes because an earlier, larger deploy left zero padding; the code is the first 1,094,544 bytes.

## Build

Toolchain used for the deployed binary: `cargo-build-sbf 4.4.0`, platform-tools `v1.57`, `solana-cli 3.0.6`.

```
cargo build-sbf --arch v0 --manifest-path programs/xfchess-game-sbf/Cargo.toml
```

The deployable build goes through the `xfchess-game-sbf` shim (same `lib.rs`, default features
`cranks`, `move-validation`, `devnet`). `nimzovich_engine` is compiled with `default-features = false`
(no_std, no search); the workspace entry must keep that setting or the engine is built with std and search.

## Reproducing the hash

A rebuild from this repository is not byte-identical to the deployed binary. It is 1,094,984 bytes, 440 more.
Dependency versions, features and the program source are the same (checked with `cargo tree`). The one known
difference is where the engine crate lives: the deployed build compiled `nimzovich_engine` from a pinned git
checkout (`xfchess-contracts` rev `67932ae`, a byte-identical copy of `crates/nimzovich_engine`), so the source
paths embedded in panic strings differ from the workspace-local ones (`crates\nimzovich_engine\src\…`). That
accounts for part of the size difference (72 bytes of path prefix were measured); the remainder has not been
attributed, so treat the two binaries as the same code but not as a verified byte-for-byte reproduction.
To compare against the on-chain hash, build from the XFChess commit above, where the deployed binary was produced.
