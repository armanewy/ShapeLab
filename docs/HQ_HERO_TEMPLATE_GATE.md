# HQ Hero Template Gate

The HQ benchmark profile for Wave 39 is:

```bash
cargo run -p shape-cli -- hq-quality-benchmark --profile prepared-hero-template-v1 --out-dir target/hq-benchmark/prepared-hero-template-v1
```

The profile writes the standard JSON evidence files where meaningful:

- `prepared-template-contract.json`
- `mesh-stats.json`
- `semantic-parts.json`
- `candidate-report.json`
- `controls-visibility-report.json`
- `export-reopen-report.json`
- `quality-report.json`

It intentionally does not write preview PNGs because no authored clay hero mesh
renderer exists yet.

## Expected Status

The expected automated result is Draft, even though the prepared template
contract itself validates. The report must state:

- no clay preview is available;
- no contact sheet is available;
- no front, three-quarter, side, back, wireframe, or silhouette view is
  available;
- no visible-control pixel-difference evidence is available;
- six direction candidates are not available;
- export and reopen are Unsupported;
- novice catalog exposure is false by default.

Unsupported outputs include:

- prepared hero clay mesh preview;
- prepared hero contact sheet;
- prepared hero export package;
- arbitrary mesh import;
- Dota/IP reconstruction;
- materials, UVs, rigging, and animation.

## Promotion Rule

Prepared Hero Template v1 cannot be promoted to Usable or Showcase from schema
validation alone. Usable requires whole-character clay previews, contact-sheet
evidence, visible control differences, six coherent direction candidates, mesh
validation, and export/reopen proof. Showcase additionally requires human
approval, contact-sheet evidence, and adversarial visual review.
