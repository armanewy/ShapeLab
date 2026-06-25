# Prepared Hero Template v1

Wave 39 adds `prepared-hero-template-v1`, a prepared-template contract for a
stylized clay hero family. It is a known-base template descriptor, not an
arbitrary character import path and not a third-party character reconstruction.

## Contract

The template records:

- the prepared humanoid template ID;
- the character base topology library version and fingerprint;
- exact base fingerprints from the versioned character base library;
- hero landmarks, semantic regions, deformation cages, and weight sets;
- future provider slots for headgear, shoulders, torso armor, belt/skirt,
  gauntlets, boots, weapon, back accessory, and hair/head mass;
- six novice-facing whole-character controls;
- a quality gate profile and human review manifest;
- explicit unsupported operations.

The primary controls are:

- Body Proportions
- Head Shape
- Garment / Armor Fit
- Pose Preset
- Silhouette
- Detail Level

These labels are the product-facing surface. Cage IDs, landmark IDs, semantic
region IDs, provider IDs, scalar paths, and raw mesh terms must not appear in
the default asset-user UI.

## Validation

`PreparedHeroTemplate::validate` rejects:

- missing or stale base topology fingerprints;
- missing or unrecognized topology versions;
- landmark, region, cage, weight-set, provider-slot, or control references that
  do not bind to the prepared known-base contract;
- missing required landmarks or semantic regions;
- more than seven primary controls;
- product-facing control labels that expose internal cage, landmark, weight,
  region, mesh, or vertex terms;
- external mesh claims;
- incomplete unsupported-operation inventories;
- Usable or Showcase quality claims from v1 evidence.

The template schema uses `serde(deny_unknown_fields)`, so hidden geometry fields
such as raw vertex payloads are rejected by deserialization.

## Scope Boundary

Prepared Hero Template v1 is a Prototype contract. It does not generate a clay
hero mesh, contact sheet, export package, UVs, materials, rig, animation, or
marketplace payload. It also does not reconstruct Dota or any other third-party
character IP.

The next product layer may attach authored providers and render whole-character
clay previews from this contract, but Wave 39 only establishes the validated
prepared-template foundation.
