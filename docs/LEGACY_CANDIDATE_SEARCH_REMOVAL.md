# Legacy Candidate Search Removal

The product-visible opaque generated variation UI is retired for the active
primitive Make workflow.

Direct primitive and Orchard-facing Make paths must not expose random or opaque
candidate generation. The old "Try ideas", candidate tray, selected-candidate
comparison, accept/reject actions, focused-part idea controls, and stale
candidate-result warnings are no longer part of the default product surface.

Deterministic primitive presets, direct bounded property edits, ObjectPlan
drafts, and contact-sheet evidence are the replacement paths. These keep the
asset source and review evidence explicit instead of asking users to choose from
opaque generated candidates.

`orchard-search-internal` remains in the workspace as an internal legacy/search crate. It
is still used by internal CLI, project-history, catalog tests, and evidence
pipelines, so this branch does not remove the crate or workspace dependency.
Those internal uses must not reintroduce product-visible candidate generation in
the active primitive Make workflow.
