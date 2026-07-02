# Object Orchard Naming Transition

Status: product-facing rename active

Object Orchard is the product name.

Migration note: the GitHub repository may still be named `ShapeLab` until the
manual repository setting is changed.

The Rust crate and folder rename has been applied. Repository host settings and
local remotes still require a manual GitHub rename step.

Feature branches must not opportunistically rename remaining durable schema IDs
or metadata fields. Those are owned by the final legacy-name purge branch.
