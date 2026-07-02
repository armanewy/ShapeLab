use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("orchard-core-legacy crate should live under crates/")
        .to_path_buf()
}

fn doc_text(relative_path: &str) -> String {
    fs::read_to_string(repo_root().join(relative_path))
        .unwrap_or_else(|error| panic!("failed to read {relative_path}: {error}"))
}

fn markdown_section<'a>(text: &'a str, heading: &str, next_heading: &str) -> &'a str {
    let start = text
        .find(heading)
        .unwrap_or_else(|| panic!("missing markdown heading {heading}"));
    let after_start = start + heading.len();
    let end = text[after_start..]
        .find(next_heading)
        .map(|offset| after_start + offset)
        .unwrap_or(text.len());
    &text[after_start..end]
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
        "crate docs should block orchard-core-legacy as the canonical product IR"
    );
    assert!(
        lib.contains("orchard-asset") && lib.contains("orchard-ir"),
        "crate docs should route future semantics to orchard-asset / orchard-ir"
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
fn orchard_core_legacy_docs_do_not_claim_canonical_a_j_ir() {
    let lib = include_str!("../src/lib.rs");
    let boundary = doc_text("docs/ORCHARD_CORE_LEGACY_BOUNDARY.md");
    let contracts = doc_text("docs/CONTRACT_BOUNDARIES.md");
    let joined = format!("{lib}\n{boundary}\n{contracts}").to_ascii_lowercase();

    assert!(
        joined.contains("shapedocument` is not the canonical a-j product ir")
            || joined.contains("shapedocument is not the canonical a-j product ir")
            || joined.contains("not the new canonical product ir"),
        "orchard-core-legacy docs should explicitly reject ShapeDocument as canonical A-J IR"
    );

    for forbidden in [
        "shapedocument is the canonical a-j product ir",
        "orchard-core-legacy::shapedocument is the canonical a-j product ir",
        "orchard-core-legacy is the canonical a-j product ir",
        "orchard-core-legacy owns the canonical a-j product ir",
        "canonical a-j product ir is shapedocument",
    ] {
        assert!(
            !joined.contains(forbidden),
            "orchard-core-legacy docs must not claim canonical A-J ownership: {forbidden}"
        );
    }
}

#[test]
fn orchard_asset_canonical_lane_remains_documented() {
    let status = doc_text("docs/CURRENT_PRODUCT_STATUS.md");
    let contracts = doc_text("docs/CONTRACT_BOUNDARIES.md");
    let architecture = doc_text("docs/ARCHITECTURE_STATUS.md");

    for (name, text) in [
        ("CURRENT_PRODUCT_STATUS", status.as_str()),
        ("CONTRACT_BOUNDARIES", contracts.as_str()),
        ("ARCHITECTURE_STATUS", architecture.as_str()),
    ] {
        assert!(
            text.contains("orchard-asset::AssetRecipe") && text.contains("Orchard IR"),
            "{name} should name orchard-asset::AssetRecipe / Orchard IR as the semantic lane"
        );
        assert!(
            text.to_ascii_lowercase().contains("canonical semantic")
                || text.to_ascii_lowercase().contains("canonical lane"),
            "{name} should keep the orchard-asset lane canonical"
        );
    }
}

#[test]
fn product_status_allowed_claims_do_not_point_users_to_shape_document() {
    let status = doc_text("docs/CURRENT_PRODUCT_STATUS.md");
    let allowed_claims = markdown_section(
        &status,
        "## Allowed Product Claims",
        "## Current Milestone Sequence",
    )
    .to_ascii_lowercase();

    for forbidden in [
        "orchard-core-legacy",
        "shapedocument",
        "legacy/implicit",
        "implicit/sdf",
        "raw document mutation",
    ] {
        assert!(
            !allowed_claims.contains(forbidden),
            "product-facing allowed claims must not point users to ShapeDocument: {forbidden}"
        );
    }

    let status_lower = status.to_ascii_lowercase();
    for forbidden in [
        "users should use shapedocument",
        "start from shapedocument",
        "make workflow uses shapedocument",
        "orchard-core-legacy is the product backbone",
        "orchard-core-legacy::shapedocument is the product backbone",
    ] {
        assert!(
            !status_lower.contains(forbidden),
            "product status must not route users to orchard-core-legacy ShapeDocument: {forbidden}"
        );
    }
}

#[test]
fn orchard_core_legacy_boundary_docs_route_a_j_semantics_elsewhere() {
    let orchard_core_legacy_boundary = doc_text("docs/ORCHARD_CORE_LEGACY_BOUNDARY.md");
    let contract_boundaries = doc_text("docs/CONTRACT_BOUNDARIES.md");
    let joined =
        format!("{orchard_core_legacy_boundary}\n{contract_boundaries}").to_ascii_lowercase();

    assert!(
        joined.contains("orchard-core-legacy::shapedocument is not the canonical a-j product ir")
            || joined.contains("shapedocument` is legacy/implicit compatibility"),
        "boundary docs should block ShapeDocument as canonical A-J IR"
    );
    assert!(
        joined.contains("orchard-asset::assetrecipe") && joined.contains("orchard-ir"),
        "boundary docs should route canonical semantics to orchard-asset / orchard-ir"
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
fn orchard_core_legacy_boundary_rejects_blocked_canonical_claims() {
    let lib = include_str!("../src/lib.rs").to_ascii_lowercase();
    let docs = doc_text("docs/ORCHARD_CORE_LEGACY_BOUNDARY.md").to_ascii_lowercase();
    let joined = format!("{lib}\n{docs}");

    for forbidden in [
        "shapedocument is the canonical a-j product ir",
        "orchard-core-legacy owns objectplan approval",
        "orchard-core-legacy owns terrain readiness",
        "orchard-core-legacy owns export readiness",
        "orchard-core-legacy owns public catalog publishing",
        "orchard-core-legacy owns game-ready status",
        "runtime llm behavior is approved",
        "public catalog publishing is approved",
        "game-ready output is approved",
    ] {
        assert!(
            !joined.contains(forbidden),
            "orchard-core-legacy boundary must not claim blocked semantic ownership: {forbidden}"
        );
    }
}
