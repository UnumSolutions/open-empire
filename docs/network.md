# Development lockstep protocol v1

The relay accepts UTF-8 JSON messages terminated by a newline, maximum 64 KiB including delimiter. There is one configured match per process and 1–8 slots. Bind defaults to loopback. Use a trusted LAN for testing; this service has no production authentication, encryption, public directory or reconnect.

1. Each connection sends `{"Hello":{"protocol":1,"identity":{"engine":"0.1.0","ruleset":"original-0.1","content_hash":"<current original_hash()>"},"player":0}}` with its distinct slot.
2. Server sends `Welcome`; when all slots join, it broadcasts `Ready`.
3. Every slot submits `{"Submit":{"tick":0,"commands":[]}}`, including empty turns.
4. Once every slot has submitted, the relay broadcasts `{"Turn":{"tick":0,"commands":[...]}}` sorted by player then sequence. Clients apply one simulation step and submit their next turn.
5. Clients may report `{"Hash":{"tick":1,"hash":"<world.hash()>"}}`. Conflicting hashes generate `Desync` with per-player hashes.

The relay binds commands to the slot assigned to that connection. It checks tick/sequence/order bounds; simulation validates ownership and gameplay legality. It does not simulate hidden game state or provide competitive anti-cheat.

Current development behavior: handshake timeout 10 seconds, peer read timeout 30 seconds, write timeout 5 seconds; any peer disconnection ends the relay. Production reconnect and pause/continue semantics from the design are not implemented.
