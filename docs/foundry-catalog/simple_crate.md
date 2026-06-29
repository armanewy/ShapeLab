# Simple Crate

Simple Crate is a primitive clay crate family for proving the Make loop and basic family authoring. It is intentionally plain: rectangular crate body, raised lid, visible lid seam, readable edge softness, one trim band, and feet or skids.

## Primary Controls

- Proportions: compact, wide storage, tall supply, and low flat crate bodies.
- Lid Height: changes the raised lid thickness.
- Edge Softness: changes rounded body and lid edges.
- Trim Thickness: changes the simple band around the crate.
- Feet Style: switches between low skids, block feet, and full runners.

Topology-changing controls are discrete choices. Scalar controls are shown with product labels only; no scalar paths or provider IDs are exposed to the product surface.

## Candidate Strategies

- Compact Box
- Wide Storage Crate
- Tall Supply Crate
- Low Flat Crate
- Reinforced Simple Crate
- Clean Minimal Crate

At least four generated ideas are expected to be visibly distinct in pure clay.

## Scope

Included geometry is limited to body, lid, lid seam rails, trim band, feet or skids, and a simple underside support only as part of the full-runner feet option.

Not included: vents, handles, fastener noise, sci-fi rails, material looks, surface authoring, natural-language modeling, external DCC integration, rigging, or animation metadata.

## Evidence

This branch writes dogfood evidence to `target/simple-crate-primitive-v0/`:

- `parent.png`
- `candidate-contact-sheet.png`
- `control-endpoint-sheet.png`
- `dogfood-summary.json`

The profile is novice-catalog visible only because the Simple Crate primitive v0 quality evidence passes the starter-template gate: five readable controls, at least four distinct ideas, no TooSubtle whole-asset candidates, no floating/broken parts, and clean export conformance.
