# Contributing

Use Rust 1.89.0. Keep authoritative simulation free from renderer types, wall-clock time, platform-dependent floating-point calculations, and unordered iteration. All player/AI actions must pass through validated commands.

Before submitting:

```sh
cargo fmt --all --check
cargo test --locked
cargo check --locked -p empire-client
```

Add behavioral tests for simulation rules and trust-boundary changes. Replays and save/load continuation should reproduce state hashes. Keep public tests self-contained and free of proprietary game files.

By contributing code you offer it under MIT OR Apache-2.0. Original media contributions use CC BY 4.0 and must include editable sources and attribution. No contributor license assignment is requested.

Report concrete implementation limitations honestly in `docs/status.md`. Do not mark a milestone complete without its acceptance evidence, including physical-device testing where specified.
