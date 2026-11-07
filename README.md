# Voidwright

Voidwright is an online role-playing game for many players. The game world is a broken ring structure in the void. Players move on a grid of tiles.

The game is not complete. The first milestone is an offline prototype for one player.

## Requirements

- Rust 1.99 or a later version

## Commands

| Task | Command |
| --- | --- |
| Build | `cargo build --workspace` |
| Test | `cargo test --workspace` |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` |
| Format | `cargo fmt --all` |

## Layout

| Path | Contents |
| --- | --- |
| `crates/core` | Game rules. No input or output. |
| `crates/client` | Game client. |
| `docs/design.md` | Design decisions. |
| `docs/lore.md` | Lore of the game world. |

## License

Apache License 2.0. Refer to [LICENSE](LICENSE).
