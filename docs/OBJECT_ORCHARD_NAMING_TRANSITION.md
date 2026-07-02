# Object Orchard Naming Transition

Status: product-facing rename active

Object Orchard is the product name.

Migration note: the GitHub repository may still be named `ShapeLab` until the
manual repository setting is changed.

The remaining technical rename is deferred because `shape-*` appears throughout
crates, package names, internal module paths, scripts, and supporting tooling.
Renaming those surfaces opportunistically would create unnecessary review noise
and branch conflicts.

Follow-up rename work requires dedicated migration branches that cover:

- crate/package names
- executable and CLI names
- scripts and build tooling
- release and packaging metadata

Feature branches must not opportunistically rename files, crates, packages,
executables, modules, or commands. Keep `shape-*` implementation names in place
until an explicit technical rename migration owns them.
