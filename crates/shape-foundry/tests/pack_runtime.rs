use std::collections::{BTreeMap, BTreeSet};

use serde::{Serialize, de::DeserializeOwned};
use shape_asset::{
    ASSET_RECIPE_SCHEMA_VERSION, AssetEdit, AssetEditProgram, AssetId, AssetRecipe, Frame3,
    GeometryRecipe, GeometrySource, ParameterDescriptor, ParameterId, PartDefinition,
    PartDefinitionId, PartInstance, PartInstanceId, Transform3, definition_scalar_path,
};
use shape_family::{
    ASSET_FAMILY_SCHEMA_VERSION, AllowedOperationKind, AssetFamilySchema, BevelPolicy,
    ExaggerationPolicy, ExportRequirement, FamilyDefaultValue, FamilyParameterKind,
    FamilyStyleFacet, FamilyStylePolicyOverrides, LengthUnit, LengthValue, NormalizedBevelProfile,
    ParameterRange, PartPrototype, PartRole, ProfileLanguage, RepetitionPolicy, RoleMultiplicity,
    RoleProportion, RoleProvision, RuntimeMetadataRequirement, STYLE_KIT_SCHEMA_VERSION, StyleKit,
    SymmetryPolicy,
};
use shape_family_compile::{
    FAMILY_IMPLEMENTATION_SCHEMA_VERSION, FamilyImplementation, ParameterBinding, RecipeFragment,
    RecipeFragmentExports, STYLE_IMPLEMENTATION_SCHEMA_VERSION, ScalarTransform,
    StyleImplementation,
    identity::{CatalogContentFingerprint, ContentFingerprint, GeometryInputFingerprint},
};
use shape_foundry::{
    CatalogContentRef, ClosedInterval, ControlDivergence, ControlKind, ControlSlotBinding,
    ControlTopologyBehavior, ControlValue, CustomizerControl, CustomizerProfile,
    DomainCertification, FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION, FeasibleControlDomain,
    FoundryAssetDocument, FoundryCatalogError, FoundryCatalogLock, FoundryCatalogResolver,
    FoundryDocumentId, FoundryLock, FoundryLockMode, FoundryLockTarget,
    FoundryPackCompilationError, FoundryPackDocument, FoundryPackExportProfile, FoundryPackIssue,
    LocalRecipeOverride, LocalRecipeOverrideId, OverrideSurvivalPolicy, PackCoherencePolicy,
    ProviderOverride, ResponseCurve, SharedProviderPolicy, TouchedSemanticTarget,
    compile_foundry_pack, document_catalog_refs,
};

#[test]
fn roman_bridge_length_set_compiles_pack_report() {
    let fixture = PackFixture::roman_bridge();
    let mut pack = exact_pack("roman-bridge-lengths", &fixture);
    pack.members.insert(
        "short".to_owned(),
        fixture.member(
            "bridge-short",
            [
                ("length", ControlValue::Scalar(0.8)),
                ("radius", ControlValue::Scalar(0.06)),
            ],
        ),
    );
    pack.members.insert(
        "medium".to_owned(),
        fixture.member(
            "bridge-medium",
            [
                ("length", ControlValue::Scalar(1.2)),
                ("radius", ControlValue::Scalar(0.06)),
            ],
        ),
    );
    pack.members.insert(
        "long".to_owned(),
        fixture.member(
            "bridge-long",
            [
                ("length", ControlValue::Scalar(1.7)),
                ("radius", ControlValue::Scalar(0.06)),
            ],
        ),
    );

    let output = compile_foundry_pack(&pack, &fixture.catalog).expect("bridge pack should compile");

    assert_eq!(output.report.members.len(), 3);
    assert!(output.report.conformance_status.accepted);
    assert!(output.report.triangle_totals.total > 0);
    assert_eq!(output.report.triangle_totals.by_member.len(), 3);
    assert!(
        output
            .report
            .shared_controls
            .iter()
            .any(|control| control.control_id == "radius")
    );
    assert!(
        output
            .report
            .differences
            .iter()
            .any(|difference| difference.subject == "control_state.length")
    );
    assert!(
        output
            .report
            .visual_descriptor_spread
            .maximum_pairwise_distance
            > 0.0
    );
}

#[test]
fn sci_fi_crate_size_set_compiles_pack_report() {
    let fixture = PackFixture::scifi_crate();
    let mut pack = exact_pack("scifi-crate-sizes", &fixture);
    pack.members.insert(
        "small".to_owned(),
        fixture.member(
            "crate-small",
            [
                ("size", ControlValue::Scalar(0.7)),
                ("radius", ControlValue::Scalar(0.03)),
            ],
        ),
    );
    pack.members.insert(
        "large".to_owned(),
        fixture.member(
            "crate-large",
            [
                ("size", ControlValue::Scalar(1.5)),
                ("radius", ControlValue::Scalar(0.03)),
            ],
        ),
    );

    let output = compile_foundry_pack(&pack, &fixture.catalog).expect("crate pack should compile");

    assert_eq!(output.report.members.len(), 2);
    assert!(output.report.conformance_status.accepted);
    assert!(output.report.triangle_totals.minimum_member > 0);
    assert!(
        output
            .report
            .shared_controls
            .iter()
            .any(|control| control.control_id == "radius")
    );
    assert!(
        output
            .report
            .differences
            .iter()
            .any(|difference| difference.subject == "control_state.size")
    );
}

#[test]
fn shared_locked_provider_is_applied_to_every_member() {
    let fixture = PackFixture::roman_bridge();
    let mut pack = exact_pack("locked-provider-bridges", &fixture);
    pack.shared_provider_policy = SharedProviderPolicy::SharedExact(BTreeMap::from([(
        "body".to_owned(),
        fixture.shared_provider_ref.clone(),
    )]));
    pack.shared_locks.push(FoundryLock {
        target: FoundryLockTarget::Provider("body".to_owned()),
        mode: FoundryLockMode::Locked,
        reason: Some("pack-authored provider".to_owned()),
    });
    pack.members.insert(
        "short".to_owned(),
        fixture.member(
            "locked-short",
            [
                ("length", ControlValue::Scalar(0.9)),
                ("radius", ControlValue::Scalar(0.05)),
            ],
        ),
    );
    pack.members.insert(
        "long".to_owned(),
        fixture.member(
            "locked-long",
            [
                ("length", ControlValue::Scalar(1.6)),
                ("radius", ControlValue::Scalar(0.05)),
            ],
        ),
    );

    let output =
        compile_foundry_pack(&pack, &fixture.catalog).expect("shared provider pack should compile");

    assert!(output.report.conformance_status.accepted);
    for member in output.report.members {
        assert_eq!(
            member.provider_choices.get("body"),
            Some(&fixture.shared_provider_ref.stable_id)
        );
    }
    for member_output in output.member_outputs.values() {
        assert_eq!(
            member_output
                .document
                .provider_overrides
                .get("body")
                .map(|provider| &provider.provider_ref),
            Some(&fixture.shared_provider_ref)
        );
        assert!(
            member_output
                .document
                .foundry_locks
                .iter()
                .any(|lock| lock.target == FoundryLockTarget::Provider("body".to_owned()))
        );
    }
}

#[test]
fn pack_authored_shared_controls_are_applied_to_every_member() {
    let fixture = PackFixture::roman_bridge();
    let mut pack = exact_pack("shared-control-bridges", &fixture);
    pack.shared_controls
        .insert("radius".to_owned(), ControlValue::Scalar(0.05));
    pack.members.insert(
        "short".to_owned(),
        fixture.member(
            "shared-control-short",
            [
                ("length", ControlValue::Scalar(0.8)),
                ("radius", ControlValue::Scalar(0.12)),
            ],
        ),
    );
    pack.members.insert(
        "long".to_owned(),
        fixture.member(
            "shared-control-long",
            [("length", ControlValue::Scalar(1.6))],
        ),
    );

    let output =
        compile_foundry_pack(&pack, &fixture.catalog).expect("shared control pack should compile");

    assert!(output.report.conformance_status.accepted);
    assert!(output.report.shared_controls.iter().any(
        |control| control.control_id == "radius" && control.value == ControlValue::Scalar(0.05)
    ));
    assert!(
        !output
            .report
            .differences
            .iter()
            .any(|difference| difference.subject == "control_state.radius")
    );
    for member in output.report.members {
        assert_eq!(
            member.controls.get("radius"),
            Some(&ControlValue::Scalar(0.05))
        );
    }
}

#[test]
fn nonconforming_member_returns_pack_level_report() {
    let mut fixture = PackFixture::roman_bridge();
    require_runtime_metadata(&mut fixture, "navmesh_anchor");
    let mut pack = exact_pack("metadata-required-bridges", &fixture);
    pack.members.insert(
        "bridge".to_owned(),
        fixture.member(
            "metadata-required-bridge",
            [
                ("length", ControlValue::Scalar(1.0)),
                ("radius", ControlValue::Scalar(0.05)),
            ],
        ),
    );

    let error = compile_foundry_pack(&pack, &fixture.catalog)
        .expect_err("member final conformance should reject the pack report");

    let FoundryPackCompilationError::CoherenceFailed(report) = error else {
        panic!("expected pack coherence report");
    };
    assert_eq!(report.members.len(), 1);
    assert!(report.triangle_totals.total > 0);
    assert!(!report.members[0].conformance.accepted);
    assert!(has_pack_issue(
        &report.conformance_status.issues,
        "pack_member_conformance_rejected"
    ));
}

#[test]
fn pack_catalog_lock_is_enforced_during_member_compilation() {
    let fixture = PackFixture::roman_bridge();
    let mut pack = exact_pack("locked-catalog-bridges", &fixture);
    pack.catalog_lock = Some(FoundryCatalogLock {
        exact_refs: BTreeMap::from([
            (
                "family".to_owned(),
                fixture.document.family_content_ref.clone(),
            ),
            (
                "style".to_owned(),
                fixture.document.style_content_ref.clone(),
            ),
            (
                "family_impl".to_owned(),
                provider_ref("wrong-family-impl", 33),
            ),
        ]),
        embedded_snapshots: Vec::new(),
        compiler_version: "0.1.0".to_owned(),
        catalog_version: 1,
    });
    pack.members.insert(
        "bridge".to_owned(),
        fixture.member(
            "catalog-lock-bridge",
            [
                ("length", ControlValue::Scalar(1.0)),
                ("radius", ControlValue::Scalar(0.05)),
            ],
        ),
    );

    let error = compile_foundry_pack(&pack, &fixture.catalog)
        .expect_err("pack catalog lock mismatch should reject member compilation");

    let FoundryPackCompilationError::MemberCompilationFailed { member_id, error } = error else {
        panic!("expected member compilation failure");
    };
    assert_eq!(member_id, "bridge");
    assert!(matches!(
        *error,
        shape_foundry::FoundryCompilationError::DocumentValidationFailed(report)
            if report
                .issues
                .iter()
                .any(|issue| issue.code == "catalog_lock_ref_mismatch")
    ));
}

#[test]
fn invalid_incompatible_member_reports_pack_coherence_failure() {
    let bridge = PackFixture::roman_bridge();
    let crate_fixture = PackFixture::scifi_crate();
    let mut catalog = bridge.catalog.clone();
    catalog
        .entries
        .extend(crate_fixture.catalog.entries.clone());

    let mut pack = exact_pack("mixed-incompatible-pack", &bridge);
    pack.coherence_policy = PackCoherencePolicy::SharedFamilyOnly;
    pack.members.insert(
        "bridge".to_owned(),
        bridge.member(
            "mixed-bridge",
            [
                ("length", ControlValue::Scalar(1.0)),
                ("radius", ControlValue::Scalar(0.05)),
            ],
        ),
    );
    pack.members.insert(
        "crate".to_owned(),
        crate_fixture.member(
            "mixed-crate",
            [
                ("size", ControlValue::Scalar(1.0)),
                ("radius", ControlValue::Scalar(0.03)),
            ],
        ),
    );

    let error = compile_foundry_pack(&pack, &catalog).expect_err("mixed pack should be rejected");

    let FoundryPackCompilationError::CoherenceFailed(report) = error else {
        panic!("expected coherence failure");
    };
    assert_eq!(report.members.len(), 2);
    assert!(report.triangle_totals.total > 0);
    assert!(!report.conformance_status.accepted);
    assert!(has_pack_issue(
        &report.conformance_status.issues,
        "pack_style_facet_mismatch"
    ));
    assert!(has_pack_issue(
        &report.conformance_status.issues,
        "pack_edge_language_mismatch"
    ));
    assert!(has_pack_issue(
        &report.conformance_status.issues,
        "pack_scale_family_mismatch"
    ));
}

#[test]
fn renamed_compatible_style_and_provider_vocabulary_are_coherent() {
    let mut fixture = PackFixture::roman_bridge();
    let mut alias_document = add_compatible_renamed_style(&mut fixture);
    let mut pack = exact_pack("compatible-renamed-style", &fixture);
    pack.coherence_policy = PackCoherencePolicy::SharedFamilyOnly;
    pack.members.insert(
        "roman".to_owned(),
        fixture.member(
            "compatible-roman",
            [
                ("length", ControlValue::Scalar(0.8)),
                ("radius", ControlValue::Scalar(0.05)),
            ],
        ),
    );
    alias_document.document_id = FoundryDocumentId("compatible-roman-alt".to_owned());
    alias_document.control_state = BTreeMap::from([
        ("length".to_owned(), ControlValue::Scalar(1.4)),
        ("radius".to_owned(), ControlValue::Scalar(0.05)),
    ]);
    lock_document(&mut alias_document);
    pack.members.insert("roman_alt".to_owned(), alias_document);

    let output = compile_foundry_pack(&pack, &fixture.catalog)
        .expect("renamed compatible style/provider vocabulary should compile");

    assert!(output.report.conformance_status.accepted);
    let style_ids = output
        .report
        .members
        .iter()
        .map(|member| member.style_ref.stable_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        style_ids,
        BTreeSet::from(["roman-alt-style", "roman-style-with-compatible-aliases"])
    );
    let compiled_lock = output
        .pack
        .catalog_lock
        .as_ref()
        .expect("compiled pack should carry shared catalog lock");
    assert_eq!(
        compiled_lock.exact_refs.get("family"),
        Some(&fixture.document.family_content_ref)
    );
    assert!(!compiled_lock.exact_refs.contains_key("style"));
    assert!(!compiled_lock.exact_refs.contains_key("style_impl"));
    let recompiled = compile_foundry_pack(&output.pack, &fixture.catalog)
        .expect("compiled SharedFamilyOnly pack should recompile");
    assert!(recompiled.report.conformance_status.accepted);
    assert_eq!(
        recompiled
            .pack
            .catalog_lock
            .as_ref()
            .expect("recompiled pack should carry shared catalog lock")
            .exact_refs,
        compiled_lock.exact_refs
    );
    assert!(
        output
            .report
            .differences
            .iter()
            .any(|difference| difference.subject == "provider_overrides.body")
    );
}

#[test]
fn duplicate_geometry_uses_geometry_fingerprint_not_artifact_metadata() {
    let mut fixture = PackFixture::roman_bridge();
    let alias_ref = add_provider_alias_to_current_style(&mut fixture, "timber_body_alias");
    let mut pack = exact_pack("duplicate-provider-geometry", &fixture);
    let mut original = fixture.member(
        "duplicate-original",
        [
            ("length", ControlValue::Scalar(1.0)),
            ("radius", ControlValue::Scalar(0.05)),
        ],
    );
    original.provider_overrides.insert(
        "body".to_owned(),
        ProviderOverride {
            role: "body".to_owned(),
            provider_ref: provider_ref("timber_body", 71),
        },
    );
    lock_document(&mut original);
    let mut alias = fixture.member(
        "duplicate-alias",
        [
            ("length", ControlValue::Scalar(1.0)),
            ("radius", ControlValue::Scalar(0.05)),
        ],
    );
    alias.provider_overrides.insert(
        "body".to_owned(),
        ProviderOverride {
            role: "body".to_owned(),
            provider_ref: alias_ref,
        },
    );
    lock_document(&mut alias);
    pack.members.insert("original".to_owned(), original);
    pack.members.insert("alias".to_owned(), alias);

    let mut probe = pack.clone();
    probe.coherence_policy = PackCoherencePolicy::Custom("allow_duplicate_geometry".to_owned());
    let probe_output = compile_foundry_pack(&probe, &fixture.catalog)
        .expect("probe duplicate geometry should compile");
    let alias_parameter = *probe_output.member_outputs["alias"]
        .recipe
        .parameters
        .keys()
        .next()
        .expect("alias recipe should expose a parameter");
    pack.members
        .get_mut("alias")
        .expect("alias member")
        .local_recipe_overrides
        .push(metadata_only_lock_override(
            "alias-recipe-lock",
            alias_parameter,
        ));

    let mut allowed = pack.clone();
    allowed.coherence_policy = PackCoherencePolicy::Custom("allow_duplicate_geometry".to_owned());
    let allowed_output = compile_foundry_pack(&allowed, &fixture.catalog)
        .expect("intentional duplicate geometry should compile");
    let original_output = &allowed_output.member_outputs["original"];
    let alias_output = &allowed_output.member_outputs["alias"];
    assert_eq!(
        original_output.artifact.combined_preview.mesh,
        alias_output.artifact.combined_preview.mesh
    );
    assert_ne!(
        original_output.build_stamp.artifact_fingerprint,
        alias_output.build_stamp.artifact_fingerprint
    );

    let error = compile_foundry_pack(&pack, &fixture.catalog)
        .expect_err("unintentional duplicate geometry should reject pack");
    let FoundryPackCompilationError::CoherenceFailed(report) = error else {
        panic!("expected duplicate geometry coherence failure");
    };
    assert!(has_pack_issue(
        &report.conformance_status.issues,
        "pack_duplicate_geometry"
    ));
}

#[test]
fn deterministic_batch_generation_repeats_pack_report() {
    let fixture = PackFixture::roman_bridge();
    let mut pack = exact_pack("deterministic-bridges", &fixture);
    pack.members.insert(
        "alpha".to_owned(),
        fixture.member(
            "det-alpha",
            [
                ("length", ControlValue::Scalar(0.95)),
                ("radius", ControlValue::Scalar(0.07)),
            ],
        ),
    );
    pack.members.insert(
        "beta".to_owned(),
        fixture.member(
            "det-beta",
            [
                ("length", ControlValue::Scalar(1.45)),
                ("radius", ControlValue::Scalar(0.07)),
            ],
        ),
    );

    let first = compile_foundry_pack(&pack, &fixture.catalog).expect("first compile should pass");
    let second = compile_foundry_pack(&pack, &fixture.catalog).expect("second compile should pass");

    assert_eq!(first.report, second.report);
    assert_eq!(
        first
            .member_outputs
            .iter()
            .map(|(id, output)| (id, output.build_stamp.clone()))
            .collect::<Vec<_>>(),
        second
            .member_outputs
            .iter()
            .map(|(id, output)| (id, output.build_stamp.clone()))
            .collect::<Vec<_>>()
    );
}

#[derive(Clone, Default)]
struct TestCatalog {
    entries: BTreeMap<String, String>,
}

impl FoundryCatalogResolver for TestCatalog {
    fn resolve_catalog_content(
        &self,
        content_ref: &CatalogContentRef,
    ) -> Result<String, FoundryCatalogError> {
        self.entries
            .get(&content_ref.stable_id)
            .cloned()
            .ok_or_else(|| FoundryCatalogError::MissingContent {
                content_ref: content_ref.clone(),
            })
    }
}

struct PackFixture {
    document: FoundryAssetDocument,
    catalog: TestCatalog,
    shared_provider_ref: CatalogContentRef,
}

impl PackFixture {
    fn roman_bridge() -> Self {
        Self::new(FixtureSpec {
            family_id: "bridge",
            family_label: "Bridge",
            style_id: "roman",
            style_label: "Roman",
            family_ref_id: "bridge-family",
            style_ref_id: "roman-style",
            family_impl_ref_id: "bridge-family-impl",
            style_impl_ref_id: "roman-style-impl",
            profile_ref_id: "bridge-profile",
            dimension_slot: "length",
            dimension_label: "Span Length",
            dimension_range: (0.5, 2.0),
            dimension_default: 1.0,
            default_provider: "timber_body",
            provider_ids: &["timber_body", "stone_body"],
            base_half_extents: [1.0, 0.32, 0.22],
            base_radius: 0.05,
            curve_family: "rounded",
            bevel_segments: 3,
            bevel_profile: 0.55,
            repetition_density: 0.35,
            detail: 0.25,
            provider_ref_byte: 71,
        })
    }

    fn scifi_crate() -> Self {
        Self::new(FixtureSpec {
            family_id: "crate",
            family_label: "Crate",
            style_id: "scifi",
            style_label: "Sci-Fi",
            family_ref_id: "crate-family",
            style_ref_id: "scifi-style",
            family_impl_ref_id: "crate-family-impl",
            style_impl_ref_id: "scifi-style-impl",
            profile_ref_id: "crate-profile",
            dimension_slot: "size",
            dimension_label: "Crate Size",
            dimension_range: (0.4, 1.8),
            dimension_default: 1.0,
            default_provider: "panel_body",
            provider_ids: &["panel_body", "reinforced_body"],
            base_half_extents: [0.8, 0.55, 0.55],
            base_radius: 0.03,
            curve_family: "faceted",
            bevel_segments: 1,
            bevel_profile: 0.2,
            repetition_density: 0.85,
            detail: 0.8,
            provider_ref_byte: 91,
        })
    }

    fn new(spec: FixtureSpec<'_>) -> Self {
        let family = family_schema(&spec);
        let style = style_kit(&spec);
        let family_impl = family_implementation(&spec);
        let style_impl = style_implementation(&spec);
        let profile = customizer_profile(&spec);

        let (family_ref, family_json) =
            catalog_entry(spec.family_ref_id, ASSET_FAMILY_SCHEMA_VERSION, &family);
        let (style_ref, style_json) =
            catalog_entry(spec.style_ref_id, STYLE_KIT_SCHEMA_VERSION, &style);
        let (family_impl_ref, family_impl_json) = catalog_entry(
            spec.family_impl_ref_id,
            FAMILY_IMPLEMENTATION_SCHEMA_VERSION,
            &family_impl,
        );
        let (style_impl_ref, style_impl_json) = catalog_entry(
            spec.style_impl_ref_id,
            STYLE_IMPLEMENTATION_SCHEMA_VERSION,
            &style_impl,
        );
        let (profile_ref, profile_json) = catalog_entry(
            spec.profile_ref_id,
            shape_foundry::CUSTOMIZER_PROFILE_SCHEMA_VERSION,
            &profile,
        );

        let mut document = FoundryAssetDocument {
            schema_version: FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION,
            document_id: FoundryDocumentId(format!("{}-doc", spec.family_id)),
            family_content_ref: family_ref,
            style_content_ref: style_ref,
            family_implementation_ref: family_impl_ref,
            style_implementation_ref: style_impl_ref,
            customizer_profile_ref: profile_ref,
            control_state: BTreeMap::from([
                (
                    spec.dimension_slot.to_owned(),
                    ControlValue::Scalar(spec.dimension_default),
                ),
                ("radius".to_owned(), ControlValue::Scalar(spec.base_radius)),
            ]),
            provider_overrides: BTreeMap::new(),
            foundry_locks: Vec::new(),
            variation_state: shape_foundry::FoundryVariationState::default(),
            local_recipe_overrides: Vec::new(),
            seed: 42,
            catalog_lock: None,
            build_stamp: None,
        };
        lock_document(&mut document);

        let catalog = TestCatalog {
            entries: BTreeMap::from([
                (spec.family_ref_id.to_owned(), family_json),
                (spec.style_ref_id.to_owned(), style_json),
                (spec.family_impl_ref_id.to_owned(), family_impl_json),
                (spec.style_impl_ref_id.to_owned(), style_impl_json),
                (spec.profile_ref_id.to_owned(), profile_json),
            ]),
        };

        Self {
            document,
            catalog,
            shared_provider_ref: provider_ref(spec.default_provider, spec.provider_ref_byte),
        }
    }

    fn member<const N: usize>(
        &self,
        document_id: &str,
        controls: [(&str, ControlValue); N],
    ) -> FoundryAssetDocument {
        let mut member = self.document.clone();
        member.document_id = FoundryDocumentId(document_id.to_owned());
        member.control_state = controls
            .into_iter()
            .map(|(control_id, value)| (control_id.to_owned(), value))
            .collect();
        member.provider_overrides.clear();
        member.foundry_locks.clear();
        member.local_recipe_overrides.clear();
        member.build_stamp = None;
        lock_document(&mut member);
        member
    }
}

struct FixtureSpec<'a> {
    family_id: &'a str,
    family_label: &'a str,
    style_id: &'a str,
    style_label: &'a str,
    family_ref_id: &'a str,
    style_ref_id: &'a str,
    family_impl_ref_id: &'a str,
    style_impl_ref_id: &'a str,
    profile_ref_id: &'a str,
    dimension_slot: &'a str,
    dimension_label: &'a str,
    dimension_range: (f32, f32),
    dimension_default: f32,
    default_provider: &'a str,
    provider_ids: &'a [&'a str],
    base_half_extents: [f32; 3],
    base_radius: f32,
    curve_family: &'a str,
    bevel_segments: u32,
    bevel_profile: f32,
    repetition_density: f32,
    detail: f32,
    provider_ref_byte: u8,
}

fn exact_pack(pack_id: &str, fixture: &PackFixture) -> FoundryPackDocument {
    FoundryPackDocument::new(
        pack_id,
        fixture.document.family_content_ref.clone(),
        fixture.document.style_content_ref.clone(),
        FoundryPackExportProfile {
            profile: "game-runtime".to_owned(),
            require_all_members: true,
        },
    )
}

fn family_schema(spec: &FixtureSpec<'_>) -> AssetFamilySchema {
    AssetFamilySchema {
        schema_version: ASSET_FAMILY_SCHEMA_VERSION,
        id: spec.family_id.to_owned(),
        display_name: spec.family_label.to_owned(),
        summary: format!("{} pack runtime fixture", spec.family_label),
        part_roles: vec![PartRole {
            id: "body".to_owned(),
            display_name: "Body".to_owned(),
            required: true,
            multiplicity: RoleMultiplicity::Single,
            provision: RoleProvision::StyleRequired,
            semantic_tags: vec!["body".to_owned()],
        }],
        attachment_rules: Vec::new(),
        allowed_operations: vec![AllowedOperationKind::Primitive],
        parameter_slots: vec![
            shape_family::FamilyParameterSlot {
                id: spec.dimension_slot.to_owned(),
                label: spec.dimension_label.to_owned(),
                target_role: Some("body".to_owned()),
                kind: FamilyParameterKind::Length {
                    unit: LengthUnit::FamilyUnits,
                },
                range: Some(ParameterRange {
                    minimum: spec.dimension_range.0,
                    maximum: spec.dimension_range.1,
                    step: 0.05,
                }),
                default_value: Some(FamilyDefaultValue::Scalar(spec.dimension_default)),
                execution_policy: shape_family::ParameterExecutionPolicy::RequiredBinding,
                topology_changing: false,
            },
            shape_family::FamilyParameterSlot {
                id: "radius".to_owned(),
                label: "Corner Radius".to_owned(),
                target_role: Some("body".to_owned()),
                kind: FamilyParameterKind::Length {
                    unit: LengthUnit::FamilyUnits,
                },
                range: Some(ParameterRange {
                    minimum: 0.01,
                    maximum: 0.18,
                    step: 0.01,
                }),
                default_value: Some(FamilyDefaultValue::Scalar(spec.base_radius)),
                execution_policy: shape_family::ParameterExecutionPolicy::RequiredBinding,
                topology_changing: false,
            },
        ],
        constraints: Vec::new(),
        variant_rules: Vec::new(),
        export_requirements: Vec::new(),
        compatible_style_kits: vec![spec.style_id.to_owned()],
        tags: Vec::new(),
    }
}

fn style_kit(spec: &FixtureSpec<'_>) -> StyleKit {
    StyleKit {
        schema_version: STYLE_KIT_SCHEMA_VERSION,
        id: spec.style_id.to_owned(),
        display_name: spec.style_label.to_owned(),
        compatible_families: vec![spec.family_id.to_owned()],
        bevel_policy: BevelPolicy {
            width: LengthValue::FamilyUnits(spec.base_radius),
            segments: spec.bevel_segments,
            profile: NormalizedBevelProfile {
                normalized: spec.bevel_profile,
            },
        },
        profile_language: ProfileLanguage {
            curve_family: spec.curve_family.to_owned(),
            allowed_profiles: vec![format!("{}-profile", spec.style_id)],
            allow_asymmetry: false,
        },
        repetition: RepetitionPolicy {
            density: spec.repetition_density,
            preferred_spacing: LengthValue::FamilyUnits(1.0),
            maximum_default_count: 4,
        },
        symmetry: SymmetryPolicy {
            prefer_mirrors: false,
            allowed_axes: Vec::new(),
        },
        exaggeration: ExaggerationPolicy {
            silhouette: 0.2,
            detail: spec.detail,
        },
        family_facets: BTreeMap::from([(
            spec.family_id.to_owned(),
            FamilyStyleFacet {
                family_id: spec.family_id.to_owned(),
                proportions: vec![RoleProportion {
                    role: "body".to_owned(),
                    preferred_scale: [
                        LengthValue::FamilyUnits(spec.base_half_extents[0] * 2.0),
                        LengthValue::FamilyUnits(spec.base_half_extents[1] * 2.0),
                        LengthValue::FamilyUnits(spec.base_half_extents[2] * 2.0),
                    ],
                    taper: 0.0,
                }],
                part_prototypes: spec
                    .provider_ids
                    .iter()
                    .map(|provider| PartPrototype {
                        id: (*provider).to_owned(),
                        display_name: (*provider).to_owned(),
                        role: "body".to_owned(),
                        operation_tags: vec![AllowedOperationKind::Primitive],
                        style_tags: vec![spec.style_id.to_owned()],
                    })
                    .collect(),
                detail_modules: Vec::new(),
                policy_overrides: FamilyStylePolicyOverrides::default(),
            },
        )]),
        tags: Vec::new(),
    }
}

fn family_implementation(spec: &FixtureSpec<'_>) -> FamilyImplementation {
    FamilyImplementation {
        schema_version: FAMILY_IMPLEMENTATION_SCHEMA_VERSION,
        family_id: spec.family_id.to_owned(),
        base_recipe: AssetRecipe::new(AssetId(1), "Base"),
        parameter_bindings: vec![
            ParameterBinding::Scalar {
                slot: spec.dimension_slot.to_owned(),
                role: "body".to_owned(),
                local_path: definition_scalar_path(
                    PartDefinitionId(1),
                    "geometry.rounded_box.half_extents.x",
                ),
                transform: ScalarTransform::Direct,
            },
            ParameterBinding::Scalar {
                slot: "radius".to_owned(),
                role: "body".to_owned(),
                local_path: definition_scalar_path(
                    PartDefinitionId(1),
                    "geometry.rounded_box.radius",
                ),
                transform: ScalarTransform::Direct,
            },
        ],
        default_role_providers: BTreeMap::new(),
        fragments: BTreeMap::new(),
        attachment_bindings: Vec::new(),
    }
}

fn style_implementation(spec: &FixtureSpec<'_>) -> StyleImplementation {
    StyleImplementation {
        schema_version: STYLE_IMPLEMENTATION_SCHEMA_VERSION,
        style_kit_id: spec.style_id.to_owned(),
        family_id: spec.family_id.to_owned(),
        default_role_providers: BTreeMap::from([(
            "body".to_owned(),
            spec.default_provider.to_owned(),
        )]),
        prototypes: spec
            .provider_ids
            .iter()
            .enumerate()
            .map(|(index, provider)| {
                let mut half_extents = spec.base_half_extents;
                half_extents[1] += index as f32 * 0.08;
                (
                    (*provider).to_owned(),
                    provider_fragment(provider, half_extents, spec.base_radius),
                )
            })
            .collect(),
        detail_modules: BTreeMap::new(),
    }
}

fn provider_fragment(id: &str, half_extents: [f32; 3], radius: f32) -> RecipeFragment {
    RecipeFragment {
        schema_version: shape_family_compile::RECIPE_FRAGMENT_SCHEMA_VERSION,
        id: id.to_owned(),
        provided_role: "body".to_owned(),
        exports: RecipeFragmentExports {
            role_occurrence_roots: vec![PartInstanceId(1)],
            internal_roots: Vec::new(),
            socket_ports: Vec::new(),
            surface_ports: Vec::new(),
        },
        recipe: body_recipe(id, half_extents, radius),
    }
}

fn body_recipe(title: &str, half_extents: [f32; 3], radius: f32) -> AssetRecipe {
    let definition_id = PartDefinitionId(1);
    let instance_id = PartInstanceId(1);
    let definition = PartDefinition {
        id: definition_id,
        name: "Body".to_owned(),
        tags: BTreeSet::new(),
        geometry: GeometryRecipe {
            source: GeometrySource::RoundedBox {
                half_extents,
                radius,
            },
            operations: Vec::new(),
        },
        regions: BTreeMap::new(),
        sockets: BTreeMap::new(),
        local_pivot: Frame3::default(),
        variant_group: None,
        production_hints: None,
    };
    let instance = PartInstance {
        id: instance_id,
        definition: definition_id,
        name: "Body".to_owned(),
        parent: None,
        local_transform: Transform3::default(),
        attachment: None,
        enabled: true,
        tags: BTreeSet::new(),
        generated_by: None,
    };
    let mut recipe = AssetRecipe::new(AssetId(7), title);
    recipe.schema_version = ASSET_RECIPE_SCHEMA_VERSION;
    recipe.definitions.insert(definition_id, definition);
    recipe.instances.insert(instance_id, instance);
    recipe.root_instances.push(instance_id);
    recipe.parameters.insert(
        ParameterId(1),
        parameter(
            ParameterId(1),
            definition_scalar_path(definition_id, "geometry.rounded_box.half_extents.x"),
            "Primary Size",
            0.2,
            2.5,
        ),
    );
    recipe.parameters.insert(
        ParameterId(2),
        parameter(
            ParameterId(2),
            definition_scalar_path(definition_id, "geometry.rounded_box.radius"),
            "Corner Radius",
            0.0,
            0.18,
        ),
    );
    recipe.next_ids.part_definition = 2;
    recipe.next_ids.part_instance = 2;
    recipe.next_ids.parameter = 3;
    recipe
}

fn parameter(
    id: ParameterId,
    path: String,
    label: &str,
    minimum: f32,
    maximum: f32,
) -> ParameterDescriptor {
    ParameterDescriptor {
        id,
        path,
        label: label.to_owned(),
        group: "Form".to_owned(),
        minimum,
        maximum,
        step: 0.01,
        mutation_sigma: 0.05,
        topology_changing: false,
        beginner_description: label.to_owned(),
    }
}

fn customizer_profile(spec: &FixtureSpec<'_>) -> CustomizerProfile {
    let mut profile = CustomizerProfile::empty(spec.family_id, Some(spec.style_id.to_owned()));
    profile.controls.push(slider_control(
        spec.dimension_slot,
        spec.dimension_label,
        spec.dimension_range.0,
        spec.dimension_range.1,
    ));
    profile
        .controls
        .push(slider_control("radius", "Corner Radius", 0.01, 0.18));
    profile
}

fn slider_control(id: &str, label: &str, minimum: f32, maximum: f32) -> CustomizerControl {
    CustomizerControl {
        id: id.to_owned(),
        label: label.to_owned(),
        section: None,
        primary: true,
        visible: true,
        kind: ControlKind::ContinuousAxis {
            default: (minimum + maximum) * 0.5,
        },
        bindings: vec![ControlSlotBinding {
            slot: id.to_owned(),
            slot_policy: shape_family::ParameterExecutionPolicy::RequiredBinding,
            response: ResponseCurve::Linear,
        }],
        domain: FeasibleControlDomain {
            continuous_intervals: vec![ClosedInterval { minimum, maximum }],
            discrete_values: Vec::new(),
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::CertifiedContinuous,
        },
        topology_behavior: ControlTopologyBehavior::TopologyPreserving,
        divergence: ControlDivergence::Synced,
    }
}

fn require_runtime_metadata(fixture: &mut PackFixture, metadata_key: &str) {
    let mut family: AssetFamilySchema =
        decode_fixture_entry(fixture, &fixture.document.family_content_ref.stable_id);
    family.export_requirements.push(ExportRequirement {
        profile: "game-runtime".to_owned(),
        required_metadata: vec![RuntimeMetadataRequirement::Custom(metadata_key.to_owned())],
        triangle_budget_hint: None,
    });
    replace_fixture_family(fixture, "bridge-family-requires-runtime-metadata", &family);
}

fn add_compatible_renamed_style(fixture: &mut PackFixture) -> FoundryAssetDocument {
    let alias_style_id = "roman-alt";
    let provider_aliases = BTreeMap::from([
        ("stone_body".to_owned(), "stone_body_alt".to_owned()),
        ("timber_body".to_owned(), "timber_body_alt".to_owned()),
    ]);

    let mut family: AssetFamilySchema =
        decode_fixture_entry(fixture, &fixture.document.family_content_ref.stable_id);
    if !family
        .compatible_style_kits
        .iter()
        .any(|style_id| style_id == alias_style_id)
    {
        family.compatible_style_kits.push(alias_style_id.to_owned());
    }
    replace_fixture_family(fixture, "bridge-family-with-compatible-style", &family);

    let mut style: StyleKit =
        decode_fixture_entry(fixture, &fixture.document.style_content_ref.stable_id);
    add_style_facet_provider_aliases(&mut style, &provider_aliases);
    replace_fixture_style(fixture, "roman-style-with-compatible-aliases", &style);

    let original_style_impl: StyleImplementation = decode_fixture_entry(
        fixture,
        &fixture.document.style_implementation_ref.stable_id,
    );
    let mut alias_style = style.clone();
    alias_style.id = alias_style_id.to_owned();
    alias_style.display_name = "Roman Alt".to_owned();
    let (alias_style_ref, alias_style_json) =
        catalog_entry("roman-alt-style", STYLE_KIT_SCHEMA_VERSION, &alias_style);
    fixture
        .catalog
        .entries
        .insert("roman-alt-style".to_owned(), alias_style_json);

    let alias_style_impl = renamed_provider_style_implementation(
        &original_style_impl,
        alias_style_id,
        &provider_aliases,
    );
    let (alias_style_impl_ref, alias_style_impl_json) = catalog_entry(
        "roman-alt-style-impl",
        STYLE_IMPLEMENTATION_SCHEMA_VERSION,
        &alias_style_impl,
    );
    fixture
        .catalog
        .entries
        .insert("roman-alt-style-impl".to_owned(), alias_style_impl_json);

    let mut alias_document = fixture.document.clone();
    alias_document.style_content_ref = alias_style_ref;
    alias_document.style_implementation_ref = alias_style_impl_ref;
    alias_document.provider_overrides.clear();
    alias_document.build_stamp = None;
    lock_document(&mut alias_document);
    alias_document
}

fn add_provider_alias_to_current_style(
    fixture: &mut PackFixture,
    alias_provider_id: &str,
) -> CatalogContentRef {
    let aliases = BTreeMap::from([("timber_body".to_owned(), alias_provider_id.to_owned())]);
    let mut style: StyleKit =
        decode_fixture_entry(fixture, &fixture.document.style_content_ref.stable_id);
    add_style_facet_provider_aliases(&mut style, &aliases);
    replace_fixture_style(fixture, "roman-style-with-provider-alias", &style);

    let mut style_impl: StyleImplementation = decode_fixture_entry(
        fixture,
        &fixture.document.style_implementation_ref.stable_id,
    );
    add_style_implementation_provider_aliases(&mut style_impl, &aliases);
    replace_fixture_style_impl(fixture, "roman-style-impl-with-provider-alias", &style_impl);

    provider_ref(alias_provider_id, 72)
}

fn metadata_only_lock_override(id: &str, parameter: ParameterId) -> LocalRecipeOverride {
    LocalRecipeOverride {
        id: LocalRecipeOverrideId(id.to_owned()),
        base_geometry_fingerprint: GeometryInputFingerprint(ContentFingerprint([0; 32])),
        edit_program: AssetEditProgram {
            label: id.to_owned(),
            seed: 7,
            operations: vec![AssetEdit::SetLock {
                parameter,
                locked: true,
            }],
        },
        touched_targets: vec![TouchedSemanticTarget::Parameter(parameter)],
        survival_policy: OverrideSurvivalPolicy::Revalidate,
    }
}

fn add_style_facet_provider_aliases(style: &mut StyleKit, aliases: &BTreeMap<String, String>) {
    for facet in style.family_facets.values_mut() {
        for (source_id, alias_id) in aliases {
            if facet
                .part_prototypes
                .iter()
                .any(|prototype| prototype.id == *alias_id)
            {
                continue;
            }
            let mut alias = facet
                .part_prototypes
                .iter()
                .find(|prototype| prototype.id == *source_id)
                .expect("source prototype should exist")
                .clone();
            alias.id = alias_id.clone();
            alias.display_name = alias_id.clone();
            facet.part_prototypes.push(alias);
        }
        facet
            .part_prototypes
            .sort_by(|left, right| left.id.cmp(&right.id));
    }
}

fn renamed_provider_style_implementation(
    original: &StyleImplementation,
    style_id: &str,
    aliases: &BTreeMap<String, String>,
) -> StyleImplementation {
    let mut renamed = original.clone();
    renamed.style_kit_id = style_id.to_owned();
    renamed.default_role_providers = original
        .default_role_providers
        .iter()
        .map(|(role, provider)| {
            (
                role.clone(),
                aliases
                    .get(provider)
                    .cloned()
                    .unwrap_or_else(|| provider.clone()),
            )
        })
        .collect();
    renamed.prototypes = original
        .prototypes
        .iter()
        .map(|(provider_id, fragment)| {
            let alias_id = aliases
                .get(provider_id)
                .cloned()
                .unwrap_or_else(|| provider_id.clone());
            let mut alias = fragment.clone();
            alias.id = alias_id.clone();
            (alias_id, alias)
        })
        .collect();
    renamed
}

fn add_style_implementation_provider_aliases(
    style_impl: &mut StyleImplementation,
    aliases: &BTreeMap<String, String>,
) {
    for (source_id, alias_id) in aliases {
        if style_impl.prototypes.contains_key(alias_id) {
            continue;
        }
        let mut alias = style_impl
            .prototypes
            .get(source_id)
            .expect("source executable prototype should exist")
            .clone();
        alias.id = alias_id.clone();
        style_impl.prototypes.insert(alias_id.clone(), alias);
    }
}

fn replace_fixture_family(fixture: &mut PackFixture, stable_id: &str, family: &AssetFamilySchema) {
    let (family_ref, family_json) = catalog_entry(stable_id, ASSET_FAMILY_SCHEMA_VERSION, family);
    fixture.document.family_content_ref = family_ref;
    fixture
        .catalog
        .entries
        .insert(stable_id.to_owned(), family_json);
    lock_document(&mut fixture.document);
}

fn replace_fixture_style(fixture: &mut PackFixture, stable_id: &str, style: &StyleKit) {
    let (style_ref, style_json) = catalog_entry(stable_id, STYLE_KIT_SCHEMA_VERSION, style);
    fixture.document.style_content_ref = style_ref;
    fixture
        .catalog
        .entries
        .insert(stable_id.to_owned(), style_json);
    lock_document(&mut fixture.document);
}

fn replace_fixture_style_impl(
    fixture: &mut PackFixture,
    stable_id: &str,
    style_impl: &StyleImplementation,
) {
    let (style_impl_ref, style_impl_json) =
        catalog_entry(stable_id, STYLE_IMPLEMENTATION_SCHEMA_VERSION, style_impl);
    fixture.document.style_implementation_ref = style_impl_ref;
    fixture
        .catalog
        .entries
        .insert(stable_id.to_owned(), style_impl_json);
    lock_document(&mut fixture.document);
}

fn decode_fixture_entry<T: DeserializeOwned>(fixture: &PackFixture, stable_id: &str) -> T {
    serde_json::from_str(
        fixture
            .catalog
            .entries
            .get(stable_id)
            .expect("fixture catalog entry should exist"),
    )
    .expect("fixture catalog entry should decode")
}

fn lock_document(document: &mut FoundryAssetDocument) {
    document.catalog_lock = Some(FoundryCatalogLock {
        exact_refs: document_catalog_refs(document),
        embedded_snapshots: Vec::new(),
        compiler_version: "0.1.0".to_owned(),
        catalog_version: 1,
    });
}

fn provider_ref(stable_id: &str, byte: u8) -> CatalogContentRef {
    CatalogContentRef {
        stable_id: stable_id.to_owned(),
        schema_version: 1,
        fingerprint: CatalogContentFingerprint(ContentFingerprint([byte; 32])),
    }
}

fn catalog_entry<T: Serialize>(
    stable_id: &str,
    schema_version: u32,
    value: &T,
) -> (CatalogContentRef, String) {
    let canonical_json = json(value);
    let fingerprint =
        shape_foundry::catalog_content_fingerprint_from_json(stable_id, &canonical_json)
            .expect("catalog content should fingerprint");
    (
        CatalogContentRef {
            stable_id: stable_id.to_owned(),
            schema_version,
            fingerprint,
        },
        canonical_json,
    )
}

fn json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("fixture should serialize")
}

fn has_pack_issue(issues: &[FoundryPackIssue], code: &str) -> bool {
    issues.iter().any(|issue| issue.code == code)
}
