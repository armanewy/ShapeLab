# Architecture

Shape Lab separates the semantic model and candidate search from any host DCC. Blender, browsers, servers, and LLMs are outside the MVP.

```text
crates/shape-core       model graph, edit programs, constraints
crates/shape-field      CPU implicit field compiler and sampling
crates/shape-mesh       CPU meshing and OBJ export
crates/shape-search     deterministic mutations and diversity selection
crates/shape-project    revisions and project persistence
crates/shape-presets    non-humanoid procedural presets
crates/shape-render     CPU preview rendering
crates/shape-app        native eframe/egui desktop application
crates/shape-cli        headless integration and demo tooling
```

The first backend is an implicit shape graph. That is representation-specific, but its document, candidate, history, and preference-navigation concepts are object-category-independent.
