# Variation Scope / Channel Contract

Shape Lab variation requests are described by a product-safe `VariationIntent`:

- `scope`: the part of the asset that may change.
- `channels`: the kind of change being requested.
- `human_label` and `human_summary`: product-facing copy for UI and automation.

The default intent is `WholeAsset` plus `CompleteLook`. Existing projects that do not contain variation state must deserialize to that default.

## Scopes

Current and reserved scopes are:

- `WholeAsset`
- `SemanticPartGroup { group_id, display_name }`
- `MaterialSlot { slot_id, display_name }`
- `DetailZone { zone_id, display_name }`
- `RigRegion { region_id, display_name }`
- `MotionSet { motion_set_id, display_name }`
- `Custom { scope_id, display_name }`

Only `WholeAsset` and `SemanticPartGroup` are product UI concepts today. Focus Part is represented by `SemanticPartGroup` and must use human-facing labels such as Body, Panels, Vents, Deck, Supports, Base, or Shade. It must not expose arbitrary mesh internals, provider IDs, scalar paths, sockets, ports, fragments, or operation IDs.

## Channels

Current and reserved channels are:

- `CompleteLook`
- `Shape`
- `Surface`
- `Wear`
- `Detail`
- `Rig`
- `Motion`
- `Gameplay`
- `Custom { channel_id, display_name }`

The default novice UI may show Complete Looks and Shape. Surface may appear only when a real surface capability sidecar exists. Rig, Motion, Gameplay, and Custom are reserved future/internal channels and are not current product features.

## Product Rules

A user-facing variation must be perceptible in the direction card where the user chooses it. Hidden/internal geometry changes may support validity, but cannot count as a shown direction.

Shape candidates cannot pass by changing only materials. Surface candidates cannot claim shape changes. Complete Looks may vary shape and surface together when both channels are supported. Until surface payloads exist, Surface is unavailable with plain copy: "Surface options will appear after this kit has a surface pack."

This contract does not implement UV generation, texture generation, material graphs, rigging, skinning, animation, runtime LLM behavior, engine-native export, or arbitrary imported mesh editability.
