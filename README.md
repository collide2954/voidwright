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

## Run

| Task | Command |
| --- | --- |
| Start the client | `cargo run --release -p voidwright-client` |
| Save one frame as a PNG file | `cargo run --release -p voidwright-client -- --capture shot.png` |
| Set the camera for a capture | `cargo run --release -p voidwright-client -- --capture shot.png --orbit 0.7 0.45 40` |
| Measure the frame rate | `cargo run --release -p voidwright-client -- --benchmark 10` |

The three values after `--orbit` are the yaw and the pitch in radians, and the distance in tiles.

## Controls

| Input | Action |
| --- | --- |
| Left mouse button | Walk to the tile. |
| Arrow keys | Turn and tilt the camera. |
| Middle mouse button, drag | Turn and tilt the camera. |
| Mouse wheel | Zoom. |

## Layout

| Path | Contents |
| --- | --- |
| `crates/core` | Game rules. No input or output. |
| `crates/client` | Game client. |
| `docs/design.md` | Design decisions. |
| `docs/lore.md` | Lore of the game world. |
| `docs/performance.md` | Performance target, method, and results. |

## License

Apache License 2.0. Refer to [LICENSE](LICENSE).
