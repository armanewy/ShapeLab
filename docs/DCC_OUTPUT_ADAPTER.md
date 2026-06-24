# DCC Output Adapter

Wave 28 adds DCC adapter sidecars to the canonical model package. The adapter is
output-only: Shape Lab remains the semantic source of truth.

The package now includes:

- `asset-manifest.json`: canonical package manifest,
- `recipe.json`: Shape Lab source recipe,
- `parts/*.meshbin`: canonical semantic part payloads,
- `validation.json`: package validation report,
- `blender_reconstruct.py`: Blender reconstruction helper,
- `dcc-adapter.json`: DCC projection manifest,
- `dcc_rebuild.py`: DCC projection rebuild helper,
- `dcc-verification.json`: DCC projection verification sidecar.

## Source Boundary

The DCC adapter sidecars explicitly record:

```text
dcc_scene_is_source_of_truth: false
external_scene_import_supported: false
canonical_package_verified: true
```

This means:

- Customize in Shape Lab.
- Export a canonical model package.
- Project semantic parts, collections, metadata, and variant-control labels to a
  DCC.
- Rebuild or verify the DCC projection from the package.

It does not mean:

- edit freely in Blender and re-import as semantic source,
- use `.blend`, `.ma`, `.fbx`, or `.obj` as the authoritative document,
- infer new Shape Lab semantics from arbitrary DCC scene edits.

## Projected Metadata

The adapter manifest projects:

- semantic part IDs,
- object names,
- source instance and definition IDs,
- parent instance IDs,
- topology signatures,
- semantic region labels,
- DCC collection/group membership,
- variant controls as metadata only.

Variant controls are exported for downstream labeling and organization. They do
not replace the Shape Lab recipe or Foundry document.

## Rebuild Helper

`dcc_rebuild.py` reads `dcc-adapter.json` and `asset-manifest.json`, verifies the
source-of-truth boundary, and writes a deterministic `dcc_projection_report.json`.
It is intentionally a projection helper, not an import adapter.
