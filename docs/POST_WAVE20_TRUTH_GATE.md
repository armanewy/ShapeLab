# Post-Wave-20 Truth Gate

Wave 21 separates demonstrated Shape Lab capability from aspirational capability.
The goal is to keep product language honest after the strict reconstruction and
known-base character work.

## Verdict

Shape Lab should be described as a semantic asset foundry with selective strict
reconstruction, not as a general mesh-to-procedural compiler.

Demonstrated product capability:

- A novice can use Visual Foundry to create, branch, customize, pack, and export
  coherent variants from ten built-in authored profiles without opening
  Advanced Recipe. The original full-benchmark profiles are Roman Timber Bridge,
  Sci-Fi Industrial Crate, and Stylized Furniture Lamp; Wave 26 adds seven
  expansion profiles through the same Foundry catalog and compile path.
- A technical user can use the explicit Modeling Workspace and authored asset
  recipes for part-aware hard-surface assets.
- The semantic program IR can strictly verify compact modeling programs under
  the `shape-program` and `shape-program-verify` contract.
- Synthetic hard-surface recovery and known-base character recovery can prove
  exact semantic recovery only for covered grammars and public benchmark inputs.
- External clean-character descriptors can be canonicalized, classified, ranked,
  and explained, but diagnostic analysis is not strict reconstruction.

Aspirational or unsupported capability:

- Arbitrary imported triangle meshes are not semantically editable.
- External clean-character analysis does not recover a strict editable program
  unless a later strict gate proves exact known-base recovery.
- Residual buffers, dense per-vertex displacements, target-derived bases, and
  literal target mesh payloads never count as strict semantic success.
- Materials, UVs, rigging, animation, marketplace publishing, and LLM workflows
  are outside the current modeling/reconstruction boundary.

## Required Language

Use "recover exact editable program" only when strict verification passes.

Use "analyze import", "rank known bases", "diagnose unsupported input", or
"suggest a matching family" when strict verification has not passed.

Do not use "editable import" for residual-based, diagnostic-only, or
correspondence-incomplete results.

## Capability Answers

What can a novice user do today without Advanced Recipe?

- Open Asset Modeling Lab in Visual Foundry.
- Pick one of the ten built-in foundry profiles.
- Generate six Refine and six Explore directions where profile controls support
  them.
- Customize primary controls, lock traits, branch history, export one asset, or
  export a three-member pack.

What can a foundry author do today?

- Add or modify Rust-authored family profiles, style kits, provider bindings,
  conformance rules, controls, candidate search behavior, preview policies, and
  export profiles.
- Validate and exercise those profiles through the existing headless foundry
  benchmark.
- Create, validate, preview, and package a typed local Foundry Author profile
  through the `shape-cli foundry-*profile` commands.

What asset classes compile cleanly from families?

- The built-in Visual Foundry profiles compile and export through their authored
  family bindings.
- Explicit asset recipes for the Modeling Workspace compile through
  `shape-modeling`, `shape-family-compile`, and `shape-compile`.
- Character grammar artifacts compile only inside the synthetic known-base
  benchmark path today, not as a general character product workflow.

What asset classes reconstruct strictly?

- Synthetic hard-surface benchmark inputs covered by the hard-surface recovery
  gate.
- Public known-base character corpus cases covered by the known-base character
  recovery gate.

What asset classes only get diagnostics?

- External clean-character descriptors handled by `shape-inverse::external_character`.
- Same-topology mesh pairs handled by the deformation decompiler when the result
  depends on final correction data rather than strict semantic recovery.
- Arbitrary imported meshes, noisy scans, meshes with unsupported topology, and
  assets outside known grammars.

Which failures are actionable?

- Missing grammar coverage.
- Missing forward operators.
- Search limits.
- Topology mismatch.
- Position mismatch.
- Dirty or non-canonical external input.
- Missing, stale, or mismatched public fingerprints.
- Drifted, surplus, missing, or unexplained semantic correspondences.
- Forbidden residual or literal target mesh evidence.

## Demo Classification

Honest product demos:

- Create a reinforced Roman bridge in Visual Foundry.
- Create a compact vented sci-fi crate family in Visual Foundry.
- Create a tall stylized lamp with a different shade in Visual Foundry.
- Open and customize a Wave 26 expansion profile in Visual Foundry.
- Export a small family pack and reopen the generated package.

Honest research demos:

- Strict synthetic hard-surface inverse recovery.
- Strict known-base character inverse recovery.
- External clean-character canonicalization and diagnostic classification.
- Same-topology deformation decompiler replay verification.

Do not present research demos as general product import/edit workflows.
