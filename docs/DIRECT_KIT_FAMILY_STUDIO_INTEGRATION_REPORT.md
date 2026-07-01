# Direct Kit Family Studio Integration Report

Date: 2026-07-01

## Verdict

`DIRECT_KIT_FAMILY_STUDIO_V0_INTEGRATED`

Family Studio Lite v0 is integrated as a developer-gated internal preview for
creating local Direct Kits from supported current shapes. It shows what stays
the same, what can change, deterministic test results, and Draft / Personal
save outcomes without restoring generated variation trays or public publishing.

## Integrated Work

- `bbbdb8b` - Family Studio Direct Kit readiness gate
- `09a3dd3` - Direct Kit contracts
- `e2da141` - Kit Capability Adapter
- `408c678` - Personal Kit local storage
- `0564e35` - Direct Kit test runner CLI
- `8e09c30` - Family Studio Lite Direct Kit UI
- `02e3ff7` - Prototype Pack brief to Draft ObjectPlan batch

The integration branch is based on `main` after these branches were already
merged.

## UI Evidence

Evidence path: `target/family-studio-lite-ui-v0/screenshots`

| UI state | Screenshot | SHA-256 |
| --- | --- | --- |
| Default entry hidden | `01-entry-hidden-default.png` | `ef65c65991b4f6d09ae55d4125e22ae101d3ffe6b383c976e59c2c95b94a1ec1` |
| Internal preview visible | `02-entry-visible-preview.png` | `71012cbe1afc9a086bece8f5ad6486825ef6673af66a09a6076c1501cdd071cb` |
| Starting point | `03-starting-point.png` | `f0c78504da1b03a2e8e8989f3fd7dbe800cfd51c4e25131b929c34d60a4bd887` |
| What stays the same | `04-what-stays-same.png` | `5c3b0551684edbf578de729c7385908d6aa7f0e3dbeb6647ef255dd7c8aa65fc` |
| What can change | `05-what-can-change.png` | `92031d58230241b6070e863b9a42f6d791e1e6e7d1a5fde8a102c0bf421745ac` |
| Test kit result | `06-test-kit-result.png` | `6d6056233a914a41c361ffc45a6bbf086ca75ceadcad00e6099aa1aa5f185116` |
| Save Draft | `07-save-draft.png` | `5639f37211653871f495076ab3752a0f4dfcfc19b0fd392ebed6c31cbc07b246` |
| Saved Personal Kit | `08-saved-personal-kit.png` | `73d79841419a3429dedaae73f41fb84dc45eda8f352379836a8fbb161477fe82` |

The UI evidence covers the default hidden state, internal preview entry,
current Box Primitive start, fixed-property copy, changeable Width / Height /
Edge Softness controls, deterministic test output, Draft save, and Personal Kit
save. Saved kits remain local/private and review-required.

## Proof Questions

| Question | Result |
| --- | --- |
| Can Direct Kit describe a current primitive or supported composition? | Pass. Direct Kit contracts and Family Studio Lite support Box Primitive, Flat Panel Primitive, Sphere Primitive, and Panel with Knob. |
| Can the UI show what stays the same? | Pass. The drawer shows fixed shape identity and locked bounded properties. |
| Can the UI show what can change? | Pass. Capability cards come from schema-backed adapter output and can be toggled. |
| Are capabilities derived from schemas/adapters? | Pass. Family Studio Lite uses the Kit Capability Adapter and primitive property schemas. |
| Can a kit be tested without generated variations? | Pass. Test output is deterministic property, preset, composition, ObjectPlan evidence, and export-report validation. |
| Can a kit be saved as Draft or PersonalOnly? | Pass. Family Studio Lite saves Draft and PersonalOnly Direct Kits through local storage. |
| Is public catalog publishing blocked? | Pass. Direct Kit visibility rejects public/reviewed/showcase promotion in V0. |
| Are ObjectPlan and Prototype Pack outputs Draft only? | Pass. ObjectPlan evidence remains review-required, and Prototype Pack brief output emits Draft ObjectPlans. |
| Is runtime LLM absent? | Pass. No app runtime LLM integration is included. |
| Are material editor, UV editor, rigging, animation, and game-ready claims absent? | Pass. They remain blocked by docs and tests. |

## Automated Gates

| Gate | Result |
| --- | --- |
| `cargo fmt --all --check` | Pass |
| `python3 scripts/check_source_hygiene.py` | Pass |
| `cargo test -p shape-foundry direct_kit --jobs 1` | Pass |
| `cargo test -p shape-cli direct_kit --jobs 1` | Pass |
| `cargo test -p shape-cli personal_kit --jobs 1` | Pass |
| `cargo test -p shape-app family_studio_lite --jobs 1` | Pass |
| `cargo test -p shape-app foundry --jobs 1` | Pass |
| `cargo test -p shape-cli prototype_pack --jobs 1` | Pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass |
| `cargo build --release --workspace` | Pass |

## Current Allowed Status

- Family Studio Lite can create local Draft / Personal Kits from supported
  primitives and safe-anchor compositions.
- Personal Kit storage exists for local/private Direct Kits.
- Prototype Pack briefs can generate Draft ObjectPlan batches for offline
  review.
- Direct Kit test results are deterministic evidence checks, not generated
  variation trays.
- No public catalog publishing exists.
- No runtime LLM integration exists.
- No material editor, UV editor, rigging, animation, or game-ready output is
  included.

## Next Allowed Work

- Prototype Pack review UI
- Godot material import proof
- Collision/game metadata contracts
- Mechanical pivot/motion contracts

## Still Blocked

- Public catalog publishing
- Runtime LLM inside the app
- Broad material editor
- UV editor
- Rigging/animation UI
- Game-ready claims
