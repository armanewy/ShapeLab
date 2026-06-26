# Roman Bridge Foundry Profile

The Roman bridge fixture defines a real `Bridge` family paired with the
`Roman Timber Engineering` style kit. It is authored as a connected timber
crossing rather than placeholder geometry: a span assembly carries the deck,
repeated supports, braces, approach ramps, and rail courses through exported
socket ports and required attachment rules.

Primary controls are capped at seven:

- Span Length
- Deck Width
- Structural Heft
- Support Rhythm
- Bracing Style
- Railing
- Edge Finish

`Span Length`, `Deck Width`, and `Structural Heft` are continuous controls with
required executable slot bindings. `Support Rhythm`, `Railing`, and `Edge
Finish` are provider galleries with whole-model preview references. `Bracing
Style` is a whole-model choice gallery that selects brace prototypes.

Candidate strategies are:

- Light
- Balanced
- Reinforced
- Wide Crossing

Conformance coverage includes required role rows for support, span, deck,
brace, ramp, and rail, plus an optional connector role for Roman timber detail
modules. Required attachment rows cover `support_to_span`, `deck_to_span`,
`brace_to_span`, `ramp_to_deck`, and `rail_to_deck`. The profile also uses array
operations for repeated timber courses and visible endpoint differences for
every primary control.

## HQ Roman Timber Bridge

Wave 34 adds `roman-bridge-hq` as the clay-quality Roman Timber Bridge vertical
slice. It keeps the same family and style identity, but uses an HQ document,
HQ provider defaults, a required connector/detail role, and the following
product-facing controls:

- Span Length
- Deck Width
- Structural Heft
- Support Style
- Bracing Style
- Railing Style
- Detail Density

The HQ support options are Round Piles, Squared Posts, Stone Piers, and Trestle
Frames. Deck, brace, rail, and connector providers are authored as whole-model
choices rather than isolated part toggles. Bracing options are Minimal Ties, X
Brace, K Brace, and Heavy Reinforced. The seven HQ direction strategies are
Reinforced, Light Crossing, Wide Crossing, Compact Span, Stone-Pier Outpost,
Detailed Timberwork, and Minimal Span.

Prompt 4 reauthors the HQ fixture for card-size clay legibility:

- support providers now separate pile rhythm, paired squared posts, masonry
  piers, and trestle frames;
- brace providers now produce distinct minimal, X, K, and heavy-reinforced
  structures;
- rail providers have clearer height/course differences;
- connector/detail providers separate clean cross ties, bolted joinery, and
  dense fasteners;
- Explore candidates are tested to return six selectable whole-asset directions
  with at least four distinct rendered-signature silhouettes.

The HQ benchmark output is written to
`target/hq-benchmark/roman-bridge-hq`; Prompt 4 evidence is expected under
`target/foundry-benchmark/roman-bridge-hq`. In this branch, Roman Bridge catalog
tests pass and include export/reopen proof, but fresh CLI visual evidence is
blocked until the unrelated `shape-cli` compile error in
`crates/shape-cli/src/game_ready_static.rs` is resolved. Default novice-catalog
exposure remains blocked until manual review is approved.
