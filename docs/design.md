# Design

This document gives the design decisions for Voidwright.

## Goals

- The game is fast.
- The game is beautiful.
- The game is easy to play on computers with few resources.
- The game is immersive.

## Game

| Topic | Decision |
| --- | --- |
| View | Low-poly 3D models on a grid of tiles. The camera turns around the player. |
| Time | The server changes the world in ticks of 600 ms. A player walks 1 tile in each tick or runs 2 tiles in each tick. |
| Running | Running uses run energy. Energy recovers when the player walks or stops. Piloting makes the recovery faster. |
| Setting | A broken ring structure in a nebula. The world is a set of floating fragments. Bridges and gates connect the fragments. |
| Progression | There are no classes. Each player can increase each skill from level 1 to level 99. |
| Combat | Attacks occur on ticks. There are three styles: Melee, Ballistics, and Tech. Each style is strong against one style and weak against the other style. |
| Economy | Exchange terminals match buy offers and sell offers. Players can also trade directly. |
| Player combat | Players can attack players only on the outer fragments. |
| Death | The player keeps the 3 items that have the highest value. The other items stay at a beacon for 15 minutes. On the outer fragments, the killer gets the other items. |

## Skills

| Skill | Group | Use |
| --- | --- | --- |
| Bladework | Combat | Accuracy of Melee attacks. |
| Brawn | Combat | Damage of Melee attacks. |
| Marksmanship | Combat | Accuracy of Ballistics attacks. |
| Ordnance | Combat | Damage of Ballistics attacks. |
| Targeting | Combat | Accuracy of Tech attacks. |
| Overload | Combat | Damage of Tech attacks. |
| Shielding | Combat | Defense against all attacks. |
| Vitality | Combat | Health points. |
| Salvaging | Gathering | Get metal, parts, and cores from wrecks and asteroid hulks. |
| Harvesting | Gathering | Grow and cut void moss, crystal stands, and hydroponic crops. |
| Trawling | Gathering | Catch creatures that drift in the void. |
| Hunting | Gathering | Track and trap void animals for hides, glands, and pets. |
| Fabrication | Production | Make gear, weapons, and tools from salvage. |
| Engineering | Production | Build bridges, devices, and a personal station. Repair broken parts of the ring. |
| Synthesis | Production | Mix stims, medkits, and void chemicals. |
| Circuitry | Production | Make implants, chips, and the tech charges for Tech attacks. |
| Piloting | Support | Move across debris courses. Recover run energy faster and use shortcuts. |
| Hacking | Support | Open locks, terminals, and drones for loot and hidden routes. |
| Provisioning | Support | Cook rations. Rations heal in combat. |
| Resonance | Support | Use void signals for short combat blessings. Blessings use resonance points. |

The total XP for level 80 is half of the total XP for level 99.

## Legions

| Topic | Decision |
| --- | --- |
| Legions | There are three fixed legions. The Wardens repair the ring to its original condition. The Reclaimers remove parts from the ring for profit and freedom. The Ascendants use void technology to change into a new form. |
| Membership | Each player pledges to one legion. There are no player groups. |
| Change of legion | A player can change legion one time in 4 weeks. A change sets the legion rank of the player to zero. |
| Territory | The outer fragments form a ring of sectors. Each legion holds one continuous arc of sectors. There are three fronts: one front between each pair of legions. |
| Conquest | Players can capture the core of a sector at a front at any time. A capture moves that front by one sector. |
| Rewards | The legion that holds a sector gets a part of the salvage from that sector. Its members pay less for fabrication at the stations in that sector. The sector shows the banners of the legion. |

## Activities

| Topic | Decision |
| --- | --- |
| Goals | Quests tell the story. Each quest has puzzles, unique rewards, and skill requirements. A contract board gives tasks that repeat. |
| Solo activities | Signal hunts: a chain of clues leads across the ring to a cache of rare items. Wreck dives: short generated dungeons in old hulls for one player. Debris courses: courses across floating debris for a movement skill. Solo bosses: hard fights for one player, with unique items. |
| Group activities | Breach defense: a team defends the core of a station from swarms. World events: meteor strikes and leviathans that all players near them can join. Raids: dungeons with many stages for a group. Role teams: each player has one role in a team challenge. |
| Lobbies | Each group activity has a launch bay in the world. Players meet at the bay and start together. |

## Look and feel

| Topic | Decision |
| --- | --- |
| Art direction | Cinematic and stylized: clean shapes with film-like light, soft sun shadows, glow on lights, and deep atmosphere. |
| Palette | Moss, soil, and rock have natural, muted colors. Neon colors are only for lights, visors, crystals, and the nebula: deep indigo void, magenta and teal clouds, and amber starlight. |
| Effects | Sun shadows, filmic color with bloom, detail textures that code paints at start, moss tufts, and drifting spores. |
| Camera | The camera follows the player. The arrow keys and the middle mouse button turn and tilt the camera. The mouse wheel changes the zoom. A left click moves the player. |
| Interface | A panel with tabs on the right side: inventory with 28 slots, skills, and equipment. A minimap at the top right. Chat at the bottom left. |
| Models | Code makes all models. Models have flat faces and vertex colors. Models do not use textures. |
| Ground | Each tile has 8 × 8 small faces with smooth shading. Soft patches of moss, soil, and bloom color the ground. Steep faces show bare rock. |
| Island edges | A jagged rock lip surrounds each island. The walls below the lip show the layers of the slab. |
| Fragment shape | Each fragment is a thick, uneven slab from the ring. Soil lies on layers of rock and Wright hull plates. The edges are broken, and the bottom is rough. The thickness changes much from fragment to fragment. |
| Fragment details | Broken girders and conduits come out of the walls and hang below. Roots and strands of moss hang over the edges. Small pieces of rock and plating float near each fragment. Thin streams of water fall from some edges and become mist in the void. |
| Avatar | A sculpted low-poly figure in a salvage suit: tapered limbs, a round helmet with a teal visor around the front, shoulder and knee plates, a backpack with tanks, and glowing seams. The faces are flat-shaded. |
| Animation | A full walk cycle: knees and elbows bend, the feet move with the ground, the hips sway, the torso leans and turns against the hips, and the body bobs two times in each stride. Running leans more and makes the stride longer. The figure breathes when it stands. |
| Walk marker | A small teal ring pulses on the target tile until the player arrives. |
| Sky | Deep indigo with magenta and teal nebula clouds and many stars. Vael glows amber low on the horizon. Far fragments of the ring drift in the distance. |
| Maps | A seed makes each island: outline, hills, rocks, and crystals. |

## Immersion

| Topic | Decision |
| --- | --- |
| Interface style | The panels look like projections from the suit of the voidwright: soft teal glass with thin scan lines. The panels flicker a little when the player is hit. The panels fade when the player only explores. |
| Sound | Each place has layers of ambient sound. Sounds come from their positions in the world. Footsteps change with the ground. Music becomes stronger in danger and at discoveries. |
| Creatures | Small void animals live on the islands and drift between them. Some animals run away when a player comes near. |
| Weather | Field storms move across the fragments, change the light, and make creatures hide. |
| Day and night | The Gyre turns, so islands move into the shadow of the ring. At night, the stars are brighter, plants glow, and other creatures come out. |
| Townsfolk | NPCs work, eat in the galley halls, go to shrines, and sleep on a schedule. |
| Travel | There are no loading screens. Near islands load while the player walks on the bridges. A gate shows a short ride through the void. |
| Camera | Important quest moments use short camera shots. Then the player gets the camera back. |
| Dialogue | NPCs say short lines above their heads. When the player talks to an NPC, a panel opens with a portrait and choices. There is no voice acting. |
| Hits | Small numbers rise and fade. The target flashes, and the suit display pulses. Health bars show only during combat. |
| Night light | Crystals, bloom moss, town lamps, and the suit lamp light the ground near them. Only a few lights are active at a time. |

## Technology

| Topic | Decision |
| --- | --- |
| Language | Rust, edition 2024. |
| Platforms | Windows, Linux, and macOS desktops. |
| Minimum hardware | Lenovo ThinkPad T590: Intel UHD Graphics 620, 8th generation Intel Core i5, 12 GB RAM. The target is 60 fps at 1920 × 1080. |
| Graphics | miniquad with OpenGL 3.3. |
| Math | glam. |
| Network | TCP with a binary protocol that the project defines. |
| Servers | Many worlds hold the inner islands. Each world holds a maximum of 2,000 players. One war shard holds the outer ring for all players. |
| Content | Items, NPCs, and quests are typed tables in Rust code. |
| Text | fontdue draws TTF fonts. The client contains one font file that has an open license. |
| Structure | A Cargo workspace with four crates: core, protocol, server, and client. |
| Dependencies | Add a crate only if it saves much work and has few dependencies. Do not use serde, tokio, or procedural macro crates. |

## Process

| Topic | Decision |
| --- | --- |
| Tests | Write each test before the code that it tests. |
| Commits | Each code commit contains one test cycle. Each commit message has one line in the form `scope: summary`. |
| Lints | Clippy pedantic. In CI, all warnings are errors. Unsafe code is not permitted. |
| Branches | All commits go to `main`. CI runs on each push. |
| Funding | The game is free and open source. There is no monetization at this time. |

## Milestones

1. Offline prototype: one player on one island. The player moves with mouse clicks.
