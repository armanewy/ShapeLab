# Deformation Decompiler Audit

This review focused on whether the package actually proves the claimed round trip rather than merely producing files that look plausible.

## Correctness and integrity fixes

- Replaced the biased ridge-based affine solve with a normalized pseudoinverse solve and an exact-translation fast path.
- Fixed a cross-language bit-exactness defect: JSON matrix coefficients are now normalized back to `f32`, and schema 2 defines stepwise non-fused binary32 affine arithmetic for both Rust and Python.
- Added strict in-memory result validation before package output.
- Added independent on-disk package replay through `verify_decompile_package` and the `verify-decompile` CLI command.
- Added validation for coordinate and numeric conventions, settings, operator ordering and IDs, affine matrix semantics, operator statistics, correction counts, and duplicate/no-op correction entries.
- Made `verification.json` part of the verified contract instead of an unchecked duplicate.
- Compare complete ordered topology arrays directly; the FNV value is now described only as a fingerprint.
- Reject package-relative traversal and symlink escapes.
- Assemble packages in a verified sibling staging directory before replacing an existing output directory.
- Tightened OBJ face-token parsing so malformed trailing-slash forms are rejected.

## Blender adapter fixes

- Validate the same package invariants in Python before touching the scene.
- Recompute and check affine and correction metadata.
- Create persistent point-domain vertex IDs.
- Verify the baked object, the Basis shape, every cumulative intermediate shape key, and the editable final shape key.
- Build the baked object from the replayed deformation stages rather than copying the target oracle directly.
- Add `--verify-existing` so a saved `.blend` can be reopened and checked again.
- Emit `blender-verification.json` and fail the process when verification does not pass.

## Test expansion

The decompiler test suite now covers genuine non-affine deformation, a deterministic affine-transform suite, package replacement, stale-file cleanup, corrupted residual data, tampered affine metadata, mismatched verification sidecars, path traversal, Unix symlink escape, malformed topology, and generated-script requirements.

The generated Python adapter was also syntax-compiled, its package loader/replay path was executed outside Blender with a stub `bpy` module, and 60 randomized cross-language packages—including rank-deficient and widely scaled inputs—were replayed bit-exactly.

Follow-up local verification on Blender 4.5.10 LTS created `reconstructed.blend`, reopened it with `--verify-existing`, and passed exact topology, persistent vertex ID, cumulative shape-key stage, baked object, and final bit-exact position checks.

## Toolchain correction

The workspace declares Rust 1.92 as its minimum because the pinned `eframe`/`egui` 0.34.3 dependency family requires Rust 1.92. The original `rust-version = 1.88` declaration was not truthful for a full workspace build.
