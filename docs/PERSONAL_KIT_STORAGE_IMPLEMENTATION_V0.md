# Personal Kit Storage Implementation v0

Status: local/private storage only.

This milestone persists Direct Kit Drafts and PersonalOnly kits into a local
Personal Kit store. It does not add public catalog publishing, cloud sync,
review/showcase promotion, app-wide library UI, runtime LLM integration,
material editing, UV/texturing, rigging, animation, or game-ready output.

## Layout

The store base directory contains:

```text
personal-kits/
  manifest.json
  kits/
    <kit-id>/
      kit.json
      source-object-plan.json, optional later
      preview.png, optional later
      evidence/
        ...
```

`kit.json` stores:

- `kit_id`
- `display_name`
- `source_kind`
- `source_ref`
- `direct_kit`
- `visibility`: `Draft` or `PersonalOnly`
- `novice_visible: false`
- `public_catalog_visible: false`
- `created_at`
- `updated_at`

Persisted JSON must not contain absolute paths.

## CLI

```bash
shape-cli personal-kit save \
  --kit direct-kit.json \
  --out-dir target/personal-kits-demo

shape-cli personal-kit list \
  --store target/personal-kits-demo

shape-cli personal-kit validate \
  --store target/personal-kits-demo
```

## Validation

Storage rejects public/catalog visibility, invalid kit IDs, invalid names,
missing sources, unsupported sources, raw mesh payload claims, game-ready
claims, and absolute paths in persisted files.

Full UI integration comes later.
