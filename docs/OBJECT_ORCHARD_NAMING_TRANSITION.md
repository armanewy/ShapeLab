# Object Orchard Naming Transition

Status: in-repo rename active

Object Orchard is the product name.

Migration note: the GitHub repository may still be named `ShapeLab` until the
manual repository setting is changed.

The Rust crate and folder rename has been applied. Repository host settings and
local remotes still require a manual GitHub rename step if the remote repository
has not been renamed yet.

Durable in-repo schema identifiers, fingerprint namespaces, file suffixes, and
DCC metadata fields now use Object Orchard names. Future feature branches should
not add compatibility aliases for the unreleased legacy names.

Broad `shape-` and `shape_` search hits are allowed only when they are ordinary
geometry vocabulary, such as shape deltas/readiness, or historical cleanup
records that list deleted pre-rename crate slugs. They are not Object Orchard
identity, package, command, environment, project suffix, or metadata names.
