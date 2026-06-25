# Post-Wave-20 Truth Gate

Wave 21 separates demonstrated Shape Lab capability from aspirational capability.
The goal is to keep product language honest after the strict reconstruction and
known-base character work.

## Verdict

Shape Lab should be described as a semantic asset foundry with selective strict
reconstruction, not as a general mesh-to-procedural compiler.

Demonstrated product capability:

- A novice can use Visual Foundry to create, branch, customize, pack, and export
  coherent variants from review-approved authored profiles with zero technical
  surface exposure. Automated preview/developer gates currently exercise seventeen
  built-in authored profiles. The original full-benchmark profiles are Roman
  Timber Bridge, Sci-Fi Industrial Crate, and Stylized Furniture Lamp; Wave 26
  adds seven expansion profiles through the same Foundry catalog and compile
  path, Wave 34 adds the HQ Roman Timber Bridge vertical slice behind manual
  review, Wave 38 adds five promoted gear kits behind the same review gate, and
  Wave 40 adds the hidden Hero Foundry clay MVP.
- A technical user can still exercise authored explicit asset recipes through
  core crates and headless CLI/tests, but the explicit Modeling Workspace is no
  longer a native product surface.
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
- Materials, UVs, rigging, animation, marketplace publishing, LLM geometry
  generation, and direct LLM recipe mutation are outside the current
  modeling/reconstruction boundary.

## Required Language

Use "recover exact editable program" only when strict verification passes.

Use "analyze import", "rank known bases", "diagnose unsupported input", or
"suggest a matching family" when strict verification has not passed.

Do not use "editable import" for residual-based, diagnostic-only, or
correspondence-incomplete results.

## Capability Answers

What can a novice user do today with zero technical surface exposure?

- Open Shape Lab directly into Visual Foundry.
- Pick a review-approved foundry profile; the developer/preview catalog currently
  exercises seventeen built-in profiles while default novice exposure remains
  manual-review gated.
- Generate six Refine and six Explore directions where profile controls support
  them.
- Customize primary controls, lock traits, branch history, export one asset, or
  export a three-member pack.
- Complete the default workflow with zero technical surface exposure.
- See rendered whole-model option thumbnails in the native default path. Wave 30
  treats placeholder option-card pixels as test-only panel fixtures, not as an
  acceptable product path.
- Bias future directions with local explicit preference signals from accepted or
  rejected candidates, locks/resets, exports, and pack membership. This is a
  bounded selection bias only, not semantic mutation or telemetry.

What can a foundry author do today?

- Add or modify Rust-authored family profiles, style kits, provider bindings,
  conformance rules, controls, candidate search behavior, preview policies, and
  export profiles.
- Validate and exercise those profiles through the existing headless foundry
  benchmark.
- Create, validate, preview, and package a typed local Foundry Author profile
  through the `shape-cli foundry-*profile` commands.
- Generate deterministic Foundry benchmark contact sheets and a Wave 30
  `shape-cli release-readiness` report for release review.
- Export canonical model packages with DCC output sidecars while keeping Shape
  Lab as the source of truth.

What integration surfaces exist today?

- The optional LLM command adapter can list profiles/controls, set controls,
  request bounded candidates, lock/reset, accept/reject, export, and summarize
  state through typed Foundry commands only.
- The DCC adapter emits output sidecars from canonical packages. Edited DCC
  scene import remains unsupported.
- Local preference learning records explicit local signals and contributes only
  a bounded post-validation candidate-selection bonus.

What asset classes compile cleanly from families?

- The built-in Visual Foundry profiles compile and export through their authored
  family bindings.
- Explicit asset recipes compile through `shape-modeling`,
  `shape-family-compile`, and `shape-compile` in core/headless paths.
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
