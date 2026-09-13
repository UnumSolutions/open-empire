# Implementation status

The agreed complete MVP remains the goal. This initial implementation is a prototype toward milestones 1–3; it does not complete any release qualification milestone.

## Implemented

- Six-crate Rust workspace; Bevy 0.16.1 desktop client; library/static library entry point for future iOS packaging.
- Original deterministic simulation for 1–8 player slots, 64×64 procedural land/islands/mixed maps, four resources, four ages, house population up to 200, construction and queued production.
- Two original faction names and simple bonuses: Aster building durability; Veyran cavalry durability and market exchange.
- Villagers, spearmen, archers, cavalry, rams, monks (healing), warships; town center, houses, production buildings, market, dock, tower, wonder.
- Basic melee/ranged counters and auto-acquisition, gathering, market sales, grid pathfinding with local occupied-cell avoidance, fog/exploration, conquest/resignation, wonder timer.
- Versioned saves/replays/content identities; deterministic command validation and rejected-command atomicity tests.
- Basic command-driven economic/military AI and bounded data-only objective evaluation for three prototype missions.
- Native battlefield UI with placeholder procedural sprites, selection, placement, hotkeys, camera, quicksave and replay export, preliminary touch tap/pan/pinch.
- Transport-independent lockstep coordinator and development TCP relay with framed-message bounds, version/hash checks, player-spoof rejection, and desync reports.
- Source inventory and content-manifest integrity tools; path traversal and duplicate-path rejection.

## Not yet implemented or qualified

- DE binary format conversion, baseline build verification against an actual installation, original campaigns/civilizations import, DE behavioral conformance tests.
- Full technology trees, diplomacy, formations, garrisoning, conversion, relic victory, trading routes, transports and comprehensive naval pathfinding. Market sales and monk healing are limited subsets.
- Hierarchical pathfinding, movement interpolation, projectiles, detailed animated art, editable 3D asset pipeline, audio, and polished map generation.
- Full campaign trigger interpreter, authored encounters, persistent campaign progress, and campaign completion QA. Current objectives illustrate the interface only.
- GUI multiplayer, server-side match hosting, public lobby discovery, reconnect, disconnect continuation, TLS/accounts, and production rate limiting. The relay stops on a peer failure or timeout.
- iOS Xcode app wrapper, Files picker, app-container saves, lifecycle/audio handling, physical M1 iPad testing, code signing and App Store submission. The static library target is not an installable iPad app.
- Touch selection-drag mode/control-group UI, performance targets, eight-player 1,600-unit stress qualification, and cross-architecture hash comparison results.
- Signed desktop release packages and platform support certification.

## Implementation defaults

- Original rules are intentionally simplified, not a DE balance recreation.
- Simulation tick is 10 Hz; units currently step whole grid cells.
- Original baseline state format is version 1. Unsupported identities are rejected. Experimental format changes may invalidate prototype saves.
- Desktop game is two-player human versus basic AI. Headless simulation supports 1–8.
- Local saves are relative to the launch directory (`saves/`). iPad sandbox storage is an unresolved platform task.
- Original graphics are placeholders. Do not market this build as DE-quality rendering or a compatible DE engine.

## Next gates, in order

1. Physical iPad wrapper, file import, lifecycle, audio, and profiling; bind a real Steam reference build and inspect representative formats.
2. Replace simplified combat/economy/navigation with the remaining agreed mechanics and validate complete original matches.
3. Connect network turns to the client, then implement discovery/reconnect and campaign events.
4. Fill the DE compatibility matrix and finish original art/content only after feasibility gates pass.
5. Cross-platform deterministic and performance qualification, packaging, licensing review, and signed releases.
