# Focus Part Interaction v0

Focus Part v0 exposes authored semantic part groups as product-facing modeling
targets. It is not mesh picking and it does not expose implementation IDs.

The starter groups are:

- Sci-Fi Industrial Crate: Body, Panels, Vents, Handles, Edge Trim, Fasteners
- Roman Timber Bridge/HQ: Deck, Supports, Bracing, Railing, Ramps, Fasteners
- Stylized Furniture Lamp: Base, Stem, Joints, Shade, Trim

Selecting a part group stores a replayable semantic focus. It does not mutate
geometry. Focused generation may only use controls and provider choices bound to
that group, must preserve locks, and must not pad unrelated whole-asset changes
into the board.

Surface Focus remains unavailable with plain copy:

```text
Surface options need textured previews before they can be shown.
```

Groups without focused controls remain visible only when useful and show:

```text
This part has no focused variations yet.
```

Default UI labels must stay semantic nouns such as Handles, Deck, Shade, and
Fasteners.
