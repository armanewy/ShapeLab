use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("shape-core crate should live under crates/")
        .to_path_buf()
}

fn doc_text(relative_path: &str) -> String {
    fs::read_to_string(repo_root().join(relative_path))
        .unwrap_or_else(|error| panic!("failed to read {relative_path}: {error}"))
}

#[test]
fn crate_level_docs_state_legacy_boundary() {
    let lib = include_str!("../src/lib.rs");

    assert!(
        lib.contains("legacy") && lib.contains("ShapeDocument"),
        "crate docs should describe ShapeDocument as legacy compatibility"
    );
    assert!(
        lib.contains("not the canonical A-J product IR"),
        "crate docs should block shape-core as the canonical product IR"
    );
    assert!(
        lib.contains("shape-asset") && lib.contains("shape-orchard-ir"),
        "crate docs should route future semantics to shape-asset / shape-orchard-ir"
    );
    assert!(
        lib.contains("ObjectPlan approval")
            && lib.contains("relationship contracts")
            && lib.contains("terrain readiness")
            && lib.contains("export readiness"),
        "crate docs should name blocked product semantics explicitly"
    );
}

#[test]
fn shape_core_boundary_docs_route_a_j_semantics_elsewhere() {
    let shape_core_boundary = doc_text("docs/SHAPE_CORE_LEGACY_BOUNDARY.md");
    let contract_boundaries = doc_text("docs/CONTRACT_BOUNDARIES.md");
    let joined = format!("{shape_core_boundary}\n{contract_boundaries}").to_ascii_lowercase();

    assert!(
        joined.contains("shape-core::shapedocument is not the canonical a-j product ir")
            || joined.contains("shapedocument` is legacy/implicit compatibility"),
        "boundary docs should block ShapeDocument as canonical A-J IR"
    );
    assert!(
        joined.contains("shape-asset::assetrecipe") && joined.contains("shape-orchard-ir"),
        "boundary docs should route canonical semantics to shape-asset / shape-orchard-ir"
    );

    for required in [
        "authoring operation logs",
        "relationship",
        "pattern",
        "material/surface",
        "collision",
        "terrain",
        "rigging",
        "animation",
        "public catalog publishing",
        "game-ready",
    ] {
        assert!(
            joined.contains(required),
            "boundary docs should mention blocked or rerouted semantic area: {required}"
        );
    }
}

#[test]
fn shape_core_boundary_rejects_blocked_canonical_claims() {
    let lib = include_str!("../src/lib.rs").to_ascii_lowercase();
    let docs = doc_text("docs/SHAPE_CORE_LEGACY_BOUNDARY.md").to_ascii_lowercase();
    let joined = format!("{lib}\n{docs}");

    for forbidden in [
        "shapedocument is the canonical a-j product ir",
        "shape-core owns objectplan approval",
        "shape-core owns terrain readiness",
        "shape-core owns export readiness",
        "shape-core owns public catalog publishing",
        "shape-core owns game-ready status",
        "runtime llm behavior is approved",
        "public catalog publishing is approved",
        "game-ready output is approved",
    ] {
        assert!(
            !joined.contains(forbidden),
            "shape-core boundary must not claim blocked semantic ownership: {forbidden}"
        );
    }
}
