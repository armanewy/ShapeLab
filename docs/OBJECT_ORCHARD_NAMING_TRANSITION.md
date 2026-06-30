# Object Orchard Naming Transition

Status: naming scaffold

Shape Lab is the current repository name and implementation code name.
Object Orchard is the intended product name.

The full rename is deferred because `shape-*` appears throughout crates, docs,
CLI surfaces, app code, package names, and supporting scripts. Renaming those
surfaces opportunistically would create unnecessary review noise and branch
conflicts.

Future rename work requires a dedicated migration plan that covers at least:

- crate/package names
- executable and CLI names
- app-facing labels
- documentation references
- scripts and build tooling
- release and packaging metadata

Feature branches must not opportunistically rename files, crates, packages,
executables, modules, commands, or user-visible behavior. Keep `shape-*`
implementation names in place until an explicit rename migration owns them.
