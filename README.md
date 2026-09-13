# Open Empire

An independently written, open-source Rust medieval RTS for desktop and iPad.

**Status: early playable prototype, not the complete MVP.** This repository implements an original integer-grid simulation, a native Bevy battlefield client, basic AI, campaign objective prototypes, a development LAN relay, and content inventory/integrity tools. It does **not** yet load or play Age of Empires II content. See [implementation status](docs/status.md) for exact limitations and release gates.

## Run

Rust **1.89.0**, a native compiler/linker, and a graphics-capable machine are required. Bevy is pinned to **0.16.1**, compatible with this toolchain. The committed lockfile pins dependencies.

```sh
cargo run --locked -p empire-client
```

On Linux install development packages for X11, xkbcommon, and a Vulkan-capable graphics driver. The CI workflow documents the Ubuntu build packages. On macOS install Xcode command-line tools; on Windows use the MSVC Rust toolchain and Visual Studio C++ build tools.

The original skirmish starts with three villagers and a town center. Select villagers and right-click nearby food, wood, gold, or stone. Build houses to increase population, advance ages, construct production buildings, and defeat the opposing settlement.

- Left-click selects; drag selects units; Shift adds to the selection.
- Right-click gathers, attacks a visible enemy, or moves.
- Arrow keys pan; scroll zooms; Space pauses.
- Ctrl/Cmd + 1–9 stores a control group; 1–9 recalls it.
- H/B/R/S/D select house/barracks/archery range/stable/dock placement. Select a villager, then click a visible empty tile. Escape cancels.
- The bottom toolbar builds and trains units. F5 saves state and replay in the platform application-data folder (`~/Library/Application Support/Open Empire` on macOS); F9 restores the quicksave.
- Touch: tap a friendly unit to select, then tap a destination/resource/enemy to order it. Two fingers pan and pinch zoom. Villagers/Army buttons select groups. Physical iPad validation remains outstanding.
- Right-side chapter buttons load the three **prototype** original missions. These are not finished campaign content.

## Headless verification

```sh
cargo test --locked
cargo run --locked -p empire-tools -- simulate 1200 2 match.empire-replay
cargo run --locked -p empire-tools -- replay match.empire-replay
cargo check --locked -p empire-client
```

Simulation hashes are deterministic serialized-state BLAKE3 hashes. Commands, player ordering, navigation, resources, and cooldowns use integer state. Rendering never contributes to authoritative state.

## Development LAN relay

```sh
cargo run --locked -p empire-tools -- relay 127.0.0.1:47624 2
```

This is a bounded newline-JSON protocol and a transport-independent lockstep coordinator, **not** an Internet-ready lobby service. The graphical client is currently local-only. See [network protocol](docs/network.md) for wire messages and limitations.

## Private DE reference inventory

Nothing from Microsoft's games ships here. On a machine containing your own Steam DE installation:

```sh
cargo run --locked -p empire-tools -- inventory-de /path/to/AoE2DE BUILD_ID /outside/repo/baseline.json
cargo run --locked -p empire-tools -- verify-pack /path/to/AoE2DE /outside/repo/baseline.json
```

Supply the numeric Steam build ID from your installation's app manifest. Inventory hashes source files and records the declared build; it does not establish ownership, discover the build automatically, convert formats, or enable gameplay. The current 4 GiB validation budget may require inventorying individual content subdirectories instead of an entire installation. [DE compatibility status](docs/compatibility.md).

## Architecture

| Crate | Responsibility |
|---|---|
| `empire-content` | Original rules, content identity, manifest validation |
| `empire-sim` | Deterministic world, commands, navigation, combat, saves, replay, objectives |
| `empire-ai` | Decision policy submitting the same commands as humans |
| `empire-net` | Versioned protocol, authenticated-slot command validation, lockstep barrier, state hashes |
| `empire-client` | Bevy rendering, camera, UI, keyboard/mouse and preliminary touch input |
| `empire-tools` | Headless matches, replay verification, private content inventory, pack verification, LAN relay |

## Licensing and contributions

Original code: **MIT OR Apache-2.0**. Original artwork/audio, when supplied, will use **CC BY 4.0** with editable sources and per-file attribution. Current visuals are procedural prototype silhouettes implemented in Rust, licensed with the code; no third-party game artwork or audio is bundled.

Please read [CONTRIBUTING](CONTRIBUTING.md) and [provenance policy](docs/provenance.md). Age of Empires II is a reference game owned by its respective rights holders. This project is independent and unaffiliated. “Open Empire” is a working title; it is distinct from glouw/openempire and may change.
