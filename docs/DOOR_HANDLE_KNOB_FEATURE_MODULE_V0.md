# Door Handle / Knob Feature Module v0

Status: PASS for internal feature-module evidence.

This branch adds exactly one visible feature after Hinged Panel: a clay
Handle / Knob. The resulting evidence profile is Handled Panel. It is not a
Door profile and does not claim open/close motion.

## Product Boundary

- Parent: Hinged Panel.
- New visible concept: Handle / Knob.
- Evidence profile: Handled Panel.
- Door naming remains blocked.
- Open/close motion, rigging, animation, UV/texturing, material looks, latch
  systems, frames, inset panels, and Family Studio UI remain blocked.

## Module Contract

The `handle-knob` feature module declares:

- required zones: Face and Handle Candidate Zone;
- provided role: `handle_knob`;
- owned control: `handle_knob_style`;
- candidate hook: handled-panel ideas;
- quality gates for pure-clay visibility, attachment, non-floating geometry,
  endpoint visibility, and no motion claim.

The module is behavior-bearing: it owns visible geometry, a visible control,
candidate hooks, and quality gates. It is not a nullable field placeholder.

## Geometry

The Handle / Knob is implemented as export-safe clay geometry attached to the
front face of the panel, opposite the hinge-side edge. It protrudes from the
panel face and remains inside the panel width and height bounds.

Allowed feature evidence:

- knob-like compact endpoint;
- pull-handle-like extended endpoint;
- width and height variants inherited from the panel proportions;
- hinge-edge variant retained as prior context.

Rejected in this branch:

- material stripes or decal tricks;
- handle/knob as texture;
- open/close behavior;
- rigging or animation;
- Door naming.

## Candidate Ideas

The internal Handled Panel fixture provides six ideas:

- Knob Panel
- Pull Handle Panel
- Wide Handled Panel
- Tall Handled Panel
- Clean Handled Panel
- Heavy Edge Handled Panel

At least four ideas compile to distinct whole-model geometry.

## Evidence

Generated evidence is under:

`target/door-handle-knob-feature-module-v0/`

Required artifacts:

- `hinged-panel-parent.png`
- `handled-panel-parent.png`
- `candidate-contact-sheet.png`
- `control-endpoint-sheet.png`
- `quality-report.json`

Artifact hashes:

```text
cef2bda97d3704f9eae7701b0b72d0cbe281cc6a67aaad5708555fdd23ae4e39  hinged-panel-parent.png
328d9393bc1b60e5fd99ba669b9a1d6a6baecfecfe8782181bd1358b7c9a4ef6  handled-panel-parent.png
db54ed99094a2aa8fe40288105be45342a15dd1167d339949d09123e186a4ef7  candidate-contact-sheet.png
db54ed99094a2aa8fe40288105be45342a15dd1167d339949d09123e186a4ef7  control-endpoint-sheet.png
857c966193ca49137b5e7cda4c85a561e4a8824ca43aaff96adefe7880785295  quality-report.json
```

`quality-report.json` records the endpoint, candidate, attachment, and export
checks for the generated evidence set.

## Test Results

Focused tests:

```text
cargo test -p shape-foundry flat_panel --jobs 1
cargo test -p shape-foundry-catalog --test flat_panel --jobs 1
```

Both pass on this branch before the full workspace gate.

## Next Allowed Work

If the full branch gates pass, the next branch may expose Handled Panel in the
Make loop. It must still avoid Door naming unless a human visual gate approves
that the model has earned it.
