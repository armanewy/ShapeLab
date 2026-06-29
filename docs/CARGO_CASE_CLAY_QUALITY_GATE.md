# Cargo Case Clay Quality Gate

Date: 2026-06-29

Cargo Case quality is judged in clay first. This gate separates strict mesh
readability from semantic display assistance so weak geometry cannot pass by
using color, texture, material, or label tricks.

## Pure Clay Gate

Pure Clay is the strict mesh gate:

- model reads with one neutral clay material.
- no textures.
- no semantic gray assistance.
- no decals.
- no material-color tricks.
- geometry must carry the form.

Pure Clay pass/fail must be recorded separately from Semantic Clay pass/fail.

## Semantic Clay Gate

Semantic Clay is a viewport readability aid:

- part groups may use neutral gray values.
- semantic grays are viewport display materials only.
- semantic grays are not UV/texturing/material support.
- semantic grays may clarify structure but cannot hide weak geometry.

Semantic Clay can improve part readability, but it must never be used to pass
the Pure Clay gate.

## Variation Gate

Variation evidence must show at least four visible whole-asset ideas. Every
primary control must have a visible endpoint difference.

Required checks:

- handles attached.
- vents readable.
- panel fields readable.
- detail density readable.
- no floating components.
- no broken procedural toy variants.
- fewer than six survivors is acceptable only if the profile's tier allows it
  and the report is honest.

## Evidence

Usable and Showcase tiers require contact sheets plus human/adversarial review.
Required evidence should include pure clay preview, semantic clay preview,
candidate contact sheet, control endpoint sheet, option gallery sheet, and a
quality report that records Pure Clay and Semantic Clay separately.
