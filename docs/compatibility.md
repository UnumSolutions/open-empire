# DE compatibility matrix

**No DE format is supported yet.** No local reference Steam installation was available during initial implementation. A numeric build ID provided to the inventory tool is recorded as a user declaration, not independently verified.

| Area | State | Acceptance gate |
|---|---|---|
| Source inventory / BLAKE3 hashes | Implemented | Run against the selected real build; reconcile Steam app manifest |
| Manifest paths / integrity | Implemented | Add transactional activation, archive extraction and iPad Files integration |
| Game data / civilizations / tech trees | Unsupported | Decode pinned baseline and enumerate every included definition |
| Sprites / palettes / terrain | Unsupported | Convert representative units, buildings and terrain with visual comparison |
| Audio | Unsupported | Decode samples and verify output locally |
| Random maps | Unsupported | Convert supported map scripts into bounded engine behavior |
| Scenarios / triggers | Unsupported | Decode representative scenario, implement every baseline-used opcode |
| Campaigns | Unsupported | Enumerate and play through every baseline mission |
| Original AI behavior | Not an exact-parity target | Our AI must play all supported rulesets competently |
| Official saves / replays / servers | Out of scope | Our clients use our formats and protocol |
| DLC / subsequent updates | Out of milestone | Require separate versioned profiles |

The importer must never silently mark unsupported instructions as converted. No decoder should be written against guessed structures in place of real reference fixtures. Private fixtures belong outside this repository and public CI; public tests use independently constructed synthetic data.
