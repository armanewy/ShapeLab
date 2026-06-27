# Focus Part Mode

Focus Part is a variation mode for semantic part groups. It is not arbitrary mesh editing.

The current product-safe part groups are derived from human-facing starter asset labels:

- Sci-Fi Crate: Body, Panels, Vents, Handles, Edge Trim, Fasteners
- Roman Bridge: Deck, Supports, Bracing, Railing, Ramps, Fasteners
- Lamp: Base, Stem, Joints, Shade, Trim

If an asset does not expose known human-facing part groups, the UI should hide or disable Focus Part with: "Focus Part is available when this asset exposes editable part groups."

## Rules

Focus Part uses `VariationScope::SemanticPartGroup`. Candidate generation must change the selected semantic part group or allowed material slots, and must not change unrelated major groups unless explicitly allowed by a future authored contract.

Focus Part must have selected-part visible delta above threshold or carry future detail zoom/highlight metadata. It must not expose raw provider roles, scalar paths, semantic IDs, operation IDs, sockets, ports, fragments, compiler/decompiler terms, conformance internals, or raw recipe data in default product UI.

Rig and Motion channels are reserved future/internal channels and are not current Focus Part product features.
