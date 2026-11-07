# Design

This document gives the design decisions for Voidwright.

## Game

| Topic | Decision |
| --- | --- |
| View | Low-poly 3D models on a grid of tiles. The camera turns around the player. |
| Time | The server changes the world in ticks of 600 ms. A player walks 1 tile in each tick or runs 2 tiles in each tick. |
| Setting | A broken ring structure in a nebula. The world is a set of floating fragments. Bridges and gates connect the fragments. |
| Progression | There are no classes. Each player can increase each skill from level 1 to level 99. |
| Combat | Attacks occur on ticks. There are three styles: Melee, Ballistics, and Tech. Each style is strong against one style and weak against the other style. |
| Economy | Exchange terminals match buy offers and sell offers. Players can also trade directly. |
| Player combat | Players can attack players only on the outer fragments. |
| Death | The player keeps the 3 items that have the highest value. The other items stay at a beacon for 15 minutes. On the outer fragments, the killer gets the other items. |

## Look and feel

| Topic | Decision |
| --- | --- |
| Palette | Deep indigo void, teal moss, magenta crystals, and amber starlight. |
| Camera | The camera follows the player. The arrow keys and the middle mouse button turn and tilt the camera. The mouse wheel changes the zoom. A left click moves the player. |
| Interface | A panel with tabs on the right side: inventory with 28 slots, skills, and equipment. A minimap at the top right. Chat at the bottom left. |
| Models | Code makes all models. Models have flat faces and vertex colors. Models do not use textures. |
| Maps | A seed makes each island: outline, hills, rocks, and crystals. |

## Technology

| Topic | Decision |
| --- | --- |
| Language | Rust, edition 2024. |
| Platforms | Windows, Linux, and macOS desktops. |
| Minimum hardware | Lenovo ThinkPad T590: Intel UHD Graphics 620, 8th generation Intel Core i5, 12 GB RAM. The target is 60 fps at 1920 × 1080. |
| Graphics | miniquad with OpenGL 3.3. |
| Math | glam. |
| Network | TCP with a binary protocol that the project defines. |
| Structure | A Cargo workspace with four crates: core, protocol, server, and client. |
| Dependencies | Add a crate only if it saves much work and has few dependencies. Do not use serde, tokio, or procedural macro crates. |

## Process

| Topic | Decision |
| --- | --- |
| Tests | Write each test before the code that it tests. |
| Commits | Each code commit contains one test cycle. Each commit message has one line in the form `scope: summary`. |
| Lints | Clippy pedantic. In CI, all warnings are errors. Unsafe code is not permitted. |
| Branches | All commits go to `main`. CI runs on each push. |

## Milestones

1. Offline prototype: one player on one island. The player moves with mouse clicks.
