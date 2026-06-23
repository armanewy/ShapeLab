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
