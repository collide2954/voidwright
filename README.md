# Voidwright

Voidwright is an online role-playing game for many players. The game world is a broken ring structure in the void. Players move on a grid of tiles.

The game is not complete. The offline prototype is playable: one player walks on one floating island. In the first online mode, several players walk on the island of one server.

## Requirements

- Rust 1.99 or a later version
- On Linux: the ALSA development files, for example the `libasound2-dev` package

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
| Start the model inspector | `cargo run --release -p voidwright-client --bin inspect` |
| Render the catalog of objects, materials, and scenes to `docs/catalog` | `cargo run --release -p voidwright-client --bin catalog` |

The three values after `--orbit` are the yaw and the pitch in radians, and the distance in tiles.

| Option | Effect |
| --- | --- |
| `--walk <dx> <dy>` | At start, the player walks to the tile at this offset. |
| `--capture-at <seconds>` | The capture waits for this game time. |
| `--record <frames>` | With `--capture`, the game moves 1/30 s for each frame and saves this number of frames. Each file name gets a frame number. The sound of the recording goes to a WAV file with the name of the capture file. |
| `--headphones` | The sound uses the headphones mix. The default mix is for speakers. |
| `--connect <address>` | The client joins the server at this address. Without this option, the client plays offline. |
| `--name <name>` | With `--connect`, the player has this name. The default is `Wanderer`. A name has 1 to 12 letters, digits, spaces, hyphens, or underscores. |

The client plays sound on the default output device. A capture without `--record` and a frame rate measurement play no sound.

## Server

| Task | Command |
| --- | --- |
| Start the server | `cargo run --release -p voidwright-server` |
| Join the server | `cargo run --release -p voidwright-client -- --connect 127.0.0.1:8643 --name Ash` |
| Measure the tick with 200 simulated players | `cargo run --release -p voidwright-server --example crowd -- 200` |

To play together, start the server. Then start one client for each player, with `--connect` and a different name.

| Option | Effect |
| --- | --- |
| `--bind <address>` | The server listens on this address. The default is `127.0.0.1:8643`. Use `0.0.0.0:8643` to accept players from other computers. |
| `--seed <number>` | The seed of the island. The default is 1. The offline game uses the same island. |

The server writes a line when a player joins or leaves, and every 100 ticks a line with the number of players and the slowest tick. [protocol.md](docs/protocol.md) gives the messages and the limits.

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
| `crates/protocol` | Messages between the client and the server, and their binary encoding. |
| `crates/server` | Game server. |
| `crates/client` | Game client. |
| `crates/client/assets/audio` | Recorded sound clips. `CREDITS.md` gives the author, source, and license of each clip. |
| `crates/client/assets/detail` | Scanned surface maps for cloth, wool, leather, and metal. `CREDITS.md` gives the author, source, and license of each scan. |
| `docs/audio.md` | Design of the sound and the music. |
| `docs/body.md` | Reference values for the body of the wanderer. |
| `docs/catalog` | Rendered images of the objects, the materials, and the scenes. `README.md` in the folder lists them and tells how to add an entry. |
| `docs/design.md` | Design decisions. |
| `docs/lore.md` | Lore of the game world. |
| `docs/motion.md` | Reference values for the motion of figures. |
| `docs/performance.md` | Performance target, method, and results. |
| `docs/protocol.md` | Network protocol between the client and the server. |

## License

Apache License 2.0. Refer to [LICENSE](LICENSE).
