# tournament_ix

Tournament instructions manage lifecycle, registration, matches, prizes, and tournament-scoped session keys.

## Subdirectories

| Path | Instructions |
|------|--------------|
| [lifecycle/](lifecycle/) | `initialize`, `initialize_escrow`, `initialize_shards`, `start`, `cancel`, `close_tournament` |
| [registration/](registration/) | `register`, `leave`, `bind_pass`, `issue_pass_eligibility` |
| [matches/](matches/) | `initialize_match`, `record_result`, `record_swiss_result` (+ shared [guards.rs](matches/guards.rs)) |
| [prizes/](prizes/) | `fund_prize`, `fund_sol_prize`, `claim_prize`, `distribute` (permissionless crank), share math in [ledger.rs](prizes/ledger.rs) |
| [session/](session/) | `authorize_tournament_session`, `session_create_game`, `session_join_game` |
| [shards.rs](shards.rs) | Player-list shard operations shared by the groups above |

## Invariants

- `shards` owns player-list operations: required shard count, duplicate checks, push, remove, collection, and Swiss standing initialization.
- Registration and leave update both `num_registered_players` and `player_count`.
- Registration reads the player's rating from their `PlayerProfile` (bucket from `base_time_seconds`); the caller supplies no Elo.
- `bind_pass` (organizer, before anyone registers) requires a Token-2022 pass for later registrations: the mint must have zero decimals and the `xfchess_pass` transfer hook (`XFCHESS_PASS_PROGRAM_ID`). `register` then burns one pass. The binding address is a fixed PDA, so a caller cannot skip it. Leaving refunds the entry fee but not the burned pass.
- `issue_pass_eligibility` (anyone, while registering) makes this program the pass registrar: it reads the player's own profile rating, checks it against the tournament range and CPIs `xfchess_pass` through the `pass_registrar` PDA to issue or refresh the player's eligibility record, so a pass can be transferred to them. The caller never supplies a rating.
- Leave removes active vector entries with `Vec::remove`; it must not write default pubkeys into active slots.
- Match results must name the two stored match participants.
- Prize payouts use `prizes::ledger` for place lookup, bit flags, and share math.
- `close_tournament` only closes `Completed` tournaments after all funded prize places are already claimed.
