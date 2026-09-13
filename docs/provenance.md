# Provenance and licensing

All implementation in this initial repository was independently written in Rust. No GPL source from 0 A.D., openage, or glouw/openempire was copied or translated.

Reference projects:
- https://gitea.wildfiregames.com/0ad/0ad — RTS architecture and historical-game design reference. Official project overview distinguishes GPL code from CC BY-SA assets.
- https://github.com/glouw/openempire — educational lockstep RTS reference, with a mixed license notice and proprietary Trial asset dependency.
- https://github.com/SFTtech/openage — engine and content-format research reference, GPLv3-or-later.

Researching a project does not grant permission to relicense its code. Do not submit copied/transliterated GPL implementations to this permissively licensed project. Document provenance for format research and newly contributed algorithms.

Original Rust code: MIT OR Apache-2.0. Dependencies retain their own licenses, available in their source packages. Before distributing binaries, produce third-party notices for the exact Cargo.lock graph.

No proprietary artwork, music, voice recordings, game data, campaign scripts, keys, or reference installation paths should be committed. The `private-content/` directory is ignored as an additional precaution, not as a substitute for reviewing staged files.

Original standalone art/audio will be CC BY 4.0. Each future asset must include creator, source, license, and editable original. Current procedural silhouettes are code, not a finished art pack.
