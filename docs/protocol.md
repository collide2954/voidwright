# Protocol

This document gives the network protocol between the client and the server. The protocol version is 1. The `voidwright-protocol` crate contains the encoder and the decoder.

## Connection

1. The client opens a TCP connection to the server.
2. The client sends a hello message. The hello message contains the protocol version and the player name.
3. The server sends a welcome message or a refusal message.
4. After a refusal message, the server closes the connection.
5. After a welcome message, the server sends an update message at each tick. The client can send walk orders.

A tick is 600 ms. The client or the server can close the connection at any time.

## Frames

Each message is in one frame.

| Part | Size | Contents |
| --- | --- | --- |
| Length | 2 bytes | The number of bytes in the payload. |
| Payload | 1 to 2,048 bytes | The tag of the message (1 byte), then the fields of the message. |

All numbers are unsigned integers in little-endian byte order. A tile is 4 bytes: the x coordinate (2 bytes), then the y coordinate (2 bytes). The positive x axis points east. The positive y axis points north.

## Messages from the client

| Tag | Message | Fields after the tag | Payload size |
| --- | --- | --- | --- |
| `0x01` | Hello | Protocol version (2 bytes), name length (1 byte), name (1 to 12 bytes) | 5 to 16 bytes |
| `0x02` | Walk | Target tile (4 bytes) | 5 bytes |

Rules for the name:

- The name has 1 to 12 characters.
- Each character is a letter from A to Z or from a to z, a digit, a space, a hyphen (`-`), or an underscore (`_`).
- The name does not start or end with a space.

## Messages from the server

| Tag | Message | Fields after the tag | Payload size |
| --- | --- | --- | --- |
| `0x81` | Welcome | Player id (2 bytes), island seed (8 bytes), spawn tile (4 bytes) | 15 bytes |
| `0x82` | Update | Tick number (4 bytes), number of sightings (1 byte), the sightings (7 bytes each) | 6 to 1,791 bytes |
| `0x83` | Refusal | Reason (1 byte) | 2 bytes |

The client makes the island from the seed with `Island::generate` and the island radius of the core crate.

A sighting gives the state of one player after the tick:

| Field | Size | Contents |
| --- | --- | --- |
| Player id | 2 bytes | The id from the welcome message of that player. |
| Tile | 4 bytes | The tile of the player. |
| Step | 1 byte | The direction of the next step on the path of the player. |

Step codes:

| Code | Step |
| --- | --- |
| 0 | No step. The player stands on the tile. |
| 1 | North |
| 2 | Northeast |
| 3 | East |
| 4 | Southeast |
| 5 | South |
| 6 | Southwest |
| 7 | West |
| 8 | Northwest |

Reason codes:

| Code | Reason |
| --- | --- |
| 1 | The server does not use the protocol version of the client. |
| 2 | The world is full. |
| 3 | The name does not obey the rules for names. |

## Updates

- Each update shows the players near the player that gets the update. That player is also in the update.
- A player is near when the straight distance between the two tiles is 32 tiles or less.
- The update shows the nearest players first, and a maximum of 255 players.
- The client removes the players that are not in the update.
- A player that walks moves 1 tile in each tick.

## Walk orders

The server keeps the last walk order that it got from each player. At the start of each tick, the server examines the order. The server refuses the order if one of these conditions is true:

- The straight distance from the player to the target tile is more than 64 tiles.
- The target tile is void or outside the map.
- No path goes from the player to the target tile.

The server does not tell the client about a refused order. The player continues on the old path. The client uses the same rules from the core crate, and does not send an order that the server refuses.

## Limits

| Limit | Value |
| --- | --- |
| Payload from a client | 64 bytes maximum |
| Payload from the server | 2,048 bytes maximum |
| Players in a world | 2,000 maximum |
| Time to send the hello | 5 seconds |
| Messages from a client | 16 at one time, then 4 more in each tick |
| Data that waits for a client | 64 KiB maximum |
| Connections that wait for a hello | 64 maximum. The server does not accept more connections until one of them sends its hello or closes. |

## Errors

A client or a server closes the connection when it gets data that is not correct:

- A frame has a length of 0.
- A frame has a length above the maximum. The connection closes before the payload arrives.
- A message has an unknown tag, an unknown step code, or an unknown reason code.
- A payload is shorter or longer than its message.

The server also closes the connection without a message if one of these conditions is true:

- The client sends a walk order before the hello, or a second hello.
- The client sends more messages than the limit permits.
- The client does not send the hello in time.
- The data that waits for the client is more than the limit. This occurs when the client does not read.

The server sends a refusal message and then closes the connection if one of these conditions is true:

- The hello contains a protocol version that is not 1.
- The name in the hello does not obey the rules.
- The world is full.

## Versions

The protocol version changes when a message changes. These parts do not change in a new version:

- The frame header.
- The tag of the hello message and the position of the protocol version in it.
- The refusal message.

Thus, a server can always refuse a client that uses a different version, and the client can always read the reason.
