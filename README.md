# Voidwright

Voidwright is an online role-playing game for many players. The game world is a broken ring structure in the void. Players move on a grid of tiles.

The game is not complete. The offline prototype is playable: one player walks on one floating island.

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

The three values after `--orbit` are the yaw and the pitch in radians, and the distance in tiles.

| Option | Effect |
| --- | --- |
| `--walk <dx> <dy>` | At start, the player walks to the tile at this offset. |
| `--capture-at <seconds>` | The capture waits for this game time. |
| `--record <frames>` | With `--capture`, the game moves 1/30 s for each frame and saves this number of frames. Each file name gets a frame number. The sound of the recording goes to a WAV file with the name of the capture file. |
| `--headphones` | The sound uses the headphones mix. The default mix is for speakers. |

The client plays sound on the default output device. A capture without `--record` and a frame rate measurement play no sound.

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
| `docs/audio.md` | Design of the sound and the music. |
| `docs/body.md` | Reference values for the body of the wanderer. |
| `docs/design.md` | Design decisions. |
| `docs/lore.md` | Lore of the game world. |
| `docs/motion.md` | Reference values for the motion of figures. |
| `docs/performance.md` | Performance target, method, and results. |

## License

Apache License 2.0. Refer to [LICENSE](LICENSE).
