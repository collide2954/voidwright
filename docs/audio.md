# Audio

This document gives the design of the sound and the music of Voidwright.

## Sources

| Topic | Decision |
| --- | --- |
| Synthesis | Code makes most sounds: wind, flames, creaks, impacts, interface sounds, and all music. The client has its own oscillators, noise sources, filters, envelopes, and resonators. |
| Recorded clips | A few short recordings add the sounds that synthesis does not make well: footsteps, cloth, and creaks of wood and metal. |
| Licenses | Each clip has a CC0 or a CC-BY license. A credits file gives the title, the author, the source, and the license of each clip. The game shows the credits of the CC-BY clips. |
| Format | Each clip is a 16-bit mono WAV file at 48 kHz. The client puts the clips in the binary and reads them with its own WAV reader. If the device uses a different sample rate, the client resamples the clips. |
| Variation | Each footstep and each impact has more than one take. The client does not play the same take two times in sequence. It changes the pitch and the level of each take a little. |

## Output and mixing

| Topic | Decision |
| --- | --- |
| Device | cpal sends stereo audio to the default output device. The client uses the sample rate of the device. |
| Threads | The game thread sends short commands to the audio thread: play, stop, move, and set a level. The commands go through a lock-free ring buffer with one writer and one reader. The audio callback owns all voices and all DSP. It does not lock, wait, or allocate memory. |
| Voices | A maximum of 32 voices play at the same time. When a sound starts and no voice is free, the quietest voice stops. |
| Smoothing | Changes of level, pan, and filter occur over a few milliseconds, so they do not click. |
| Buses | The mix has five buses: music, ambience, effects, interface, and master. Each bus has a volume slider in the settings. |
| Budget | Audio uses a maximum of 5% of one core of the ThinkPad T590. |

## Space

| Topic | Decision |
| --- | --- |
| Listener | The listener is at the head of the player figure and turns with the camera. |
| Positional sound | Sounds come from their positions in the world. The pan follows the direction to the source. The level decreases with distance, and a low-pass filter makes far sources duller. |
| Speakers mode | The pan is an equal-power stereo pan. |
| Headphones mode | Simple binaural cues: a time difference between the ears of a maximum of 0.65 ms, and a head-shadow filter on the far ear. The sources sound outside the head. |
| Place reverb | A feedback delay network with 8 delay lines makes the reverb. The size, the decay time, and the tone of the reverb change with the place: the open void, island hollows, and halls. |

## Ambience and effects

| Topic | Decision |
| --- | --- |
| Living ambience | Each place has layers of ambient sound. They stay far below the footsteps and are never a constant hiss. A faint, dark wind rises in slow gusts and dies down between them. Moss and spores rustle on the islands. The ring groans far away. The layers change with the place and the time of day. |
| Footsteps | Each foot strike of the gait plays a footstep. The ground under the foot selects the clips: moss, soil, rock, or metal plate. Running plays harder steps than walking. |
| Lantern | The flame of the lantern hisses quietly. The lantern creaks on its rod when it swings quickly. The swing speed of the lantern strand controls the creak. |

## Music

| Topic | Decision |
| --- | --- |
| Style | Sparse ambient music: slow drones, soft pads, and rare bowed or plucked motifs. Silence is part of the music. |
| Layers | Each place has a quiet bed of drones and pads. A motif layer fades in at a discovery, for example a new island or a landmark, and fades out after it. A low pulse layer fades in during danger. |
| Rests | The bed plays for some minutes and then stops for some minutes. |
| Notes | The note data is in Rust code. Code synthesizes all instruments. |

## Mix

| Topic | Decision |
| --- | --- |
| Speakers mode | This is the default mode. The mix aims at −16 LUFS. A gentle compressor keeps quiet details audible on small speakers. A limiter keeps the peaks below −1 dBTP. |
| Headphones mode | The mix aims at −23 LUFS with wide dynamics. Only the limiter acts on the master bus. |
| Mono | A switch folds the stereo mix to mono, for one ear or one speaker. |
| Background | When the window loses focus, the game mutes the audio. A setting can change this to a lower level. |
| Captions | Important sounds show a short caption near the chat, for example "creature growls, left". Footsteps and ambience do not show captions. |

## First slice

The first playable slice contains these sounds:

- Footsteps on each ground type.
- The void wind.
- The hiss and the creak of the lantern.
- The reverb of the open island.
- One music bed, with a motif that plays at a discovery.

## Sources

- EBU R 128, *Loudness normalisation and permitted maximum level of audio signals*.
- ITU-R BS.1770, *Algorithms to measure audio programme loudness and true-peak audio level*.
- Jot and Chaigne 1991, *Digital delay networks for designing artificial reverberators*, AES 90th Convention.
- Woodworth 1938, *Experimental Psychology*: the time difference between the ears.
