# Trim Band Feature Module v0

Date: 2026-06-30

## Verdict

Pass.

Trim Band adds exactly one visible feature after Lidded Box. The internal
preview object is:

```text
Trimmed Box = Lidded Box + Trim Band
```

It is not a crate, not a material stripe, not a panel set, not feet/skids, not
handles, and not a public Family Studio flow.

## Module Contract

Internal module:

```text
TrimBandModule
```

Requires:

- box body
- exterior edge/band placement zone

Provides:

- visible `trim_band` role
- `Trim Thickness` control
- trimmed-box candidate hook

Quality gates:

- trim band visible in pure clay
- trim does not float
- trim does not become a material stripe
- trim endpoint visibly differs

The module owns only `trim_thickness`. Box Primitive remains unchanged, and
Lidded Box remains Box Primitive plus Lid Seam only.

## Geometry

The trim band is export-safe clay geometry. It is represented as a raised
rounded-box band placed just outside the box body face, with a small clearance
to avoid accidental triangle intersections. It is not texture, color, decal, or
surface state.

## Candidate Ideas

- Clean Trimmed Box
- Reinforced Trimmed Box
- Compact Trimmed Box
- Wide Trimmed Box
- Low Trim Box
- Soft Trimmed Box

## Evidence

Generated local evidence:

```text
target/trim-band-feature-module-v0/
```

Artifacts:

- `lidded-box-parent.png`
- `trimmed-box-parent.png`
- `candidate-contact-sheet.png`
- `endpoint-sheet.png`
- `quality-report.json`

SHA-256:

```text
a54acb2f719dd2d52cb6a0b9d7930bd37b20650ddd94655d473c4230862f91a4  candidate-contact-sheet.png
c58a950321603d7d4ac5ca32bc4840220bfc62a019816f3265f8f38b5c12861d  endpoint-sheet.png
f8a11275b5f9a2b23418b0f4141f92af5faf8f72843a6cf90b34f0e2df6a17a4  lidded-box-parent.png
3c257cfc724ea92e031086c99e3dd32391cea54edf9e975d4f00075b78201955  trimmed-box-parent.png
a45155009398366223d0b544081b02d49b01aec062d87bcbc4c78b11398a90bc  quality-report.json
```

Quality report:

```text
trim_visible_in_pure_clay: true
trim_does_not_float: true
trim_not_material_stripe: true
trim_endpoint_visible: true
candidates_differ: true
avoided_disallowed_modules: true
avoided_crate_claim: true
export_clean: true
```

## Tests

Covered by `cargo test -p shape-foundry-catalog --test box_primitive --jobs 1`:

- Box Primitive without Lid Seam remains unchanged.
- Lidded Box without Trim Band remains unchanged.
- Trimmed Box includes TrimBandModule.
- Trim is visible in pure clay.
- Trim endpoint visibly differs.
- Candidate ideas compile to distinct shapes.
- No feet, panel, handle, latch, or vent modules are included.
- No crate/case language appears in Trimmed Box copy.
- Export package verification is clean.

## Next Work

Stop the box ladder. The next different-kernel proof should be Door Primitive.

Still blocked:

- crate language
- Feet / Skids
- panels
- handles
- material looks
- UV/texturing
- rigging/animation
- public Family Studio flow
