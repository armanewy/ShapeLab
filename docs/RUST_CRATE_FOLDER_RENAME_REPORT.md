# Rust Crate Folder Rename Report

Date: 2026-07-01

Status: Prompt 9 implementation branch.

## Mapping

| Old package/folder | New package/folder |
| --- | --- |
| `shape-app` | `orchard-app` |
| `shape-cli` | `orchard-cli` |
| `shape-asset` | `orchard-asset` |
| `shape-authoring` | `orchard-authoring` |
| `shape-core` | `orchard-core-legacy` |
| `shape-modeling` | `orchard-modeling` |
| `shape-modeling-assets` | `orchard-modeling-assets` |
| `shape-compile` | `orchard-compile` |
| `shape-foundry` | `orchard-foundry` |
| `shape-foundry-catalog` | `orchard-foundry-catalog` |
| `shape-render` | `orchard-render` |
| `shape-project` | `orchard-project` |
| `shape-search` | `orchard-search-internal` |
| `shape-field` | `orchard-field` |
| `shape-mesh` | `orchard-mesh` |
| `shape-family` | `orchard-family` |
| `shape-family-compile` | `orchard-family-compile` |
| `shape-poly` | `orchard-poly` |
| `shape-presets` | `orchard-presets` |

## Commands Changed

- CLI package and binary command examples now use `orchard-cli`.
- Desktop app package and binary references now use `orchard-app`.
- Cargo examples use `cargo run -p orchard-cli` and `cargo build -p orchard-app`.
- CI and local development scripts reference the renamed package and binary
  names where they invoke Cargo packages or generated binaries.

## Temporary Exceptions

- Repository URLs and local repository path strings are owned by
  `docs/OBJECT_ORCHARD_REPOSITORY_RENAME_GUIDE.md` and the manual GitHub
  repository setting step.
- Environment variables were renamed to `OBJECT_ORCHARD_*` by the repository
  path/script cleanup branch.
- Persistent schema identifiers and existing serialized project kind values
  with legacy namespace prefixes are left for the final legacy-name purge
  branch.
- Some docs retain historical cleanup branch names or deleted crate names where
  they describe already-completed cleanup work.

## Post-Rename Manual Steps

- Run the final legacy-name purge after repository settings are updated.
- Rename the GitHub repository manually in repository settings when ready.
