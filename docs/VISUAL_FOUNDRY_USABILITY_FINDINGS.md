# Visual Foundry Usability Findings

Wave 10 uses local-only usability instrumentation and deterministic headless
benchmarks. No geometry, absolute export paths, or model payloads are recorded
in usability events.

## Benchmark Findings

The original three MVP profiles pass the Wave 10 usability gate:

- Time to first build is recorded in the benchmark metrics.
- Time to first export is recorded after coherent pack export.
- Candidate survival rate is `1.0` in the local Wave 10 benchmark run.
- Invalid attempts are `0` in the local Wave 10 benchmark run.
- Advanced Recipe visits are `0` in the local Wave 10 benchmark run.
- All primary controls are measurable.
- No profile requires Advanced Recipe for the core task.

## Native UI Findings

- Wave 31 makes Visual Foundry the direct native product app instead of nesting
  it inside Asset Modeling Lab or a top-level mode switcher.
- Whole-model previews are visible in Directions and Customize.
- The default Customize surface avoids raw scalar paths; technical paths are
  not shown in the default product shell.
- Lock, reset, preview, apply, candidate choose/reject, undo, export, save, and
  pack export are all reducer-backed actions.

## Known Usability Weak Spots

- The Choose screen exposes only built-in family fixtures and `.shapelab-foundry`
  project open. Arbitrary recipe import is not a product route.
- The native panels are functional but visually dense; candidate and control
  cards should become larger, more scannable rows after the contract stabilizes.
- The current texture upload path is simple and correct, but should cache
  egui textures by preview/build ID for smoother interaction.

## Later Resolutions

- Wave 30 makes native customizer option cards a release gate: every built-in
  profile must expose rendered 64x64 whole-model option thumbnails in the
  reducer state path.
