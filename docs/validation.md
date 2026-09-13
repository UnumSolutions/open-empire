# Validation evidence

Initial implementation, September 13, 2026, local macOS / Rust 1.89.0:

- `cargo test --locked`: 14 passing behavioral tests across content, simulation, and networking.
- `cargo check --locked -p empire-client` and `cargo build --locked -p empire-client`: passed.
- Native macOS application opened and rendered the battlefield, resource HUD, action toolbar, and scenario controls. Keyboard pause was exercised. Automated pointer actions were unavailable in the desktop automation service, so mouse/touch usability is not signed off.
- Two-AI, 1,200-tick headless match recorded and independently replayed to matching BLAKE3 state hash `b8739673bdeeba827836d511b886597bf41014b3b9fdb3217d835f65fc73e05a`. This is a regression sample, not a performance qualification.
- Synthetic content inventory/verification passed, with tampered bytes and path traversal rejected.
- A real loopback TCP test connected two clients, completed three lockstep turns, rejected a spoofed player command, and broadcast conflicting state hashes.

GitHub Actions runs tests and client compilation on Linux, macOS, and Windows; a separate job compares the resulting deterministic state hashes. Read the actual workflow results for the current commit rather than assuming every platform has passed.

Not validated: physical iPad, iOS linking/signing, DE import/conversion, complete missions, GUI multiplayer, 1,600-unit performance, signed release builds, or App Store acceptance.
