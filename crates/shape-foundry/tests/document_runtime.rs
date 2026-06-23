use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use shape_asset::{
    ASSET_RECIPE_SCHEMA_VERSION, AssetEdit, AssetEditProgram, AssetId, AssetRecipe, Frame3,
    GeometryRecipe, GeometrySource, ParameterDescriptor, ParameterId, PartDefinition,
    PartDefinitionId, PartInstance, PartInstanceId, Transform3, definition_scalar_path, get_scalar,
};
use shape_family::{
    ASSET_FAMILY_SCHEMA_VERSION, AllowedOperationKind, AssetFamilySchema, BevelPolicy,
    ExaggerationPolicy, ExportRequirement, FamilyDefaultValue, FamilyParameterKind,
    FamilyStyleFacet, FamilyStylePolicyOverrides, LengthUnit, LengthValue, NormalizedBevelProfile,
    ParameterExecutionPolicy, ParameterRange, PartPrototype, PartRole, ProfileLanguage,
    RepetitionPolicy, RoleMultiplicity, RoleProvision, RuntimeMetadataRequirement,
    STYLE_KIT_SCHEMA_VERSION, StyleKit, SymmetryPolicy,
};
use shape_family_compile::{
    FAMILY_IMPLEMENTATION_SCHEMA_VERSION, FamilyImplementation, ParameterBinding, RecipeFragment,
    RecipeFragmentExports, STYLE_IMPLEMENTATION_SCHEMA_VERSION, ScalarTransform,
    StyleImplementation,
    identity::{CatalogContentFingerprint, ContentFingerprint},
};
use shape_foundry::{
    CatalogContentRef, ClosedInterval, ControlDivergence, ControlKind, ControlSlotBinding,
    ControlTopologyBehavior, ControlValue, CustomizerControl, CustomizerProfile,
    DomainCertification, EmbeddedCatalogSnapshot, FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION,
    FeasibleControlDomain, FoundryAssetDocument, FoundryCatalogError, FoundryCatalogLock,
    FoundryCatalogResolver, FoundryCommand, FoundryDocumentId, FoundryLock, FoundryLockMode,
    FoundryLockTarget, LocalOverrideApplicationStatus, LocalRecipeOverride, LocalRecipeOverrideId,
    OverrideSurvivalPolicy, ResponseCurve, compile_foundry_document, document_catalog_refs,
    replay_foundry_commands,
};

#[test]
fn deterministic_build_emits_stable_stamp_and_snapshot() {
    let fixture = RuntimeFixture::new();

    let first = compile_foundry_document(&fixture.document, &fixture.catalog)
        .expect("document should compile");
    let second = compile_foundry_document(&fixture.document, &fixture.catalog)
        .expect("document should compile deterministically");

    assert_eq!(first.build_stamp, second.build_stamp);
    assert_eq!(first.recipe_snapshot, second.recipe_snapshot);
    assert_eq!(first.document.build_stamp, Some(first.build_stamp.clone()));
    assert!(first.conformance_summary.accepted);
}

#[test]
fn catalog_fingerprint_mismatch_is_reported() {
    let mut fixture = RuntimeFixture::new();
    let mut wrong_family = family_schema();
    wrong_family.display_name = "Wrong Bridge".to_owned();
    fixture
        .catalog
        .entries
        .insert("bridge-family".to_owned(), json(&wrong_family));

    let error = compile_foundry_document(&fixture.document, &fixture.catalog)
        .expect_err("catalog mismatch should fail");

    assert!(matches!(
        error,
        shape_foundry::FoundryCompilationError::Catalog(
            FoundryCatalogError::FingerprintMismatch { lock_key, .. }
        ) if lock_key == "family"
    ));
}

#[test]
fn embedded_snapshot_recovers_when_catalog_is_absent() {
    let mut fixture = RuntimeFixture::new();
    let snapshots = fixture.snapshots_for(&fixture.document);
    fixture
        .document
        .catalog_lock
        .as_mut()
        .expect("lock")
        .embedded_snapshots = snapshots;
    let empty_catalog = TestCatalog::default();

    let output = compile_foundry_document(&fixture.document, &empty_catalog)
        .expect("embedded snapshots should recover locked content");

    assert!(
        output
            .catalog
            .resolved_content
            .values()
            .all(|content| content.source
                == shape_foundry::FoundryCatalogContentSource::EmbeddedSnapshot)
    );
    assert!(output.conformance_summary.accepted);
}

#[test]
fn final_conformance_rejects_missing_required_export_metadata() {
    let mut fixture = RuntimeFixture::new();
    let mut family = family_schema();
    family.export_requirements.push(ExportRequirement {
        profile: "game-runtime".to_owned(),
        required_metadata: vec![RuntimeMetadataRequirement::Custom(
            "navmesh_anchor".to_owned(),
        )],
        triangle_budget_hint: None,
    });
    let (family_ref, family_json) =
        catalog_entry("bridge-family", ASSET_FAMILY_SCHEMA_VERSION, &family);
    fixture.document.family_content_ref = family_ref;
    fixture.document.catalog_lock = Some(FoundryCatalogLock {
        exact_refs: document_catalog_refs(&fixture.document),
        embedded_snapshots: Vec::new(),
        compiler_version: "0.1.0".to_owned(),
        catalog_version: 1,
    });
    fixture
        .catalog
        .entries
        .insert("bridge-family".to_owned(), family_json);

    let error = compile_foundry_document(&fixture.document, &fixture.catalog)
        .expect_err("missing required metadata should reject final conformance");

    assert!(matches!(
        error,
        shape_foundry::FoundryCompilationError::FinalConformanceRejected(report)
            if report
                .exports
                .iter()
                .any(|row| row.issue_codes.contains(&"missing_required_export_metadata".to_owned()))
    ));
}

#[test]
fn document_catalog_lock_rejects_unused_exact_refs() {
    let mut fixture = RuntimeFixture::new();
    fixture
        .document
        .catalog_lock
        .as_mut()
        .expect("lock")
        .exact_refs
        .insert("unused".to_owned(), provider_ref("unused"));

    let error = compile_foundry_document(&fixture.document, &fixture.catalog)
        .expect_err("unused exact refs should reject the lock");

    assert!(matches!(
        error,
        shape_foundry::FoundryCompilationError::DocumentValidationFailed(report)
            if report
                .issues
                .iter()
                .any(|issue| issue.code == "extra_catalog_lock_ref")
    ));
}

#[test]
fn incompatible_style_is_rejected_by_family_compile() {
    let mut fixture = RuntimeFixture::new();
    fixture.switch_to_incompatible_style();

    let error = compile_foundry_document(&fixture.document, &fixture.catalog)
        .expect_err("incompatible style should fail");

    assert!(matches!(
        error,
        shape_foundry::FoundryCompilationError::FamilyCompile(_)
    ));
}

#[test]
fn revalidated_override_survives_changed_style_when_edit_still_applies() {
    let mut fixture = RuntimeFixture::new();
    let original = compile_foundry_document(&fixture.document, &fixture.catalog)
        .expect("original style should compile");
    let radius_parameter = *original
        .base_recipe
        .parameters
        .keys()
        .next()
        .expect("base recipe should expose a radius parameter");
    fixture.switch_to_modern_style();
    fixture
        .document
        .local_recipe_overrides
        .push(radius_override(
            "radius-edit",
            original.base_geometry_fingerprint,
            radius_parameter,
            OverrideSurvivalPolicy::Revalidate,
            0.3,
        ));

    let output = compile_foundry_document(&fixture.document, &fixture.catalog)
        .expect("revalidated override should compile");

    assert_eq!(
        output.local_override_reports[0].status,
        LocalOverrideApplicationStatus::Revalidated
    );
    let radius_path = output
        .recipe
        .parameters
        .get(&radius_parameter)
        .expect("radius parameter should remain addressable")
        .path
        .clone();
    assert_eq!(
        get_scalar(&output.recipe, radius_path).expect("radius should read"),
        0.3
    );
}

#[test]
fn pinned_override_rejects_changed_style() {
    let mut fixture = RuntimeFixture::new();
    let original = compile_foundry_document(&fixture.document, &fixture.catalog)
        .expect("original style should compile");
    let radius_parameter = *original
        .base_recipe
        .parameters
        .keys()
        .next()
        .expect("base recipe should expose a radius parameter");
    fixture.switch_to_modern_style();
    fixture
        .document
        .local_recipe_overrides
        .push(radius_override(
            "radius-edit",
            original.base_geometry_fingerprint,
            radius_parameter,
            OverrideSurvivalPolicy::Pinned,
            0.3,
        ));

    let error = compile_foundry_document(&fixture.document, &fixture.catalog)
        .expect_err("pinned override should reject changed base geometry");

    assert!(matches!(
        error,
        shape_foundry::FoundryCompilationError::LocalOverrideRejected {
            reason,
            ..
        } if reason == "pinned_base_geometry_changed"
    ));
}

#[test]
fn exact_recipe_snapshot_round_trips_final_recipe() {
    let fixture = RuntimeFixture::new();
    let output = compile_foundry_document(&fixture.document, &fixture.catalog)
        .expect("document should compile");

    let snapshot_recipe: AssetRecipe = serde_json::from_str(&output.recipe_snapshot.canonical_json)
        .expect("snapshot should decode");

    assert_eq!(snapshot_recipe, output.recipe);
    assert_eq!(
        output.recipe_snapshot.recipe_fingerprint,
        output.build_stamp.recipe_fingerprint
    );
}

#[test]
fn command_replay_applies_non_candidate_commands_before_compile() {
    let mut fixture = RuntimeFixture::new();
    fixture.document.control_state.clear();
    let commands = vec![
        FoundryCommand::SetControl {
            control_id: "radius".to_owned(),
            value: ControlValue::Scalar(0.2),
        },
        FoundryCommand::SelectProvider {
            role: "body".to_owned(),
            provider_ref: provider_ref("heavy_body"),
        },
        FoundryCommand::SetLock {
            lock: FoundryLock {
                target: FoundryLockTarget::Control("radius".to_owned()),
                mode: FoundryLockMode::Locked,
                reason: Some("test lock".to_owned()),
            },
        },
    ];
    let replay = replay_foundry_commands(fixture.document.clone(), &commands)
        .expect("commands should replay");

    fixture.document = replay.document;
    let output = compile_foundry_document(&fixture.document, &fixture.catalog)
        .expect("replayed document should compile");

    assert_eq!(
        output.provider_override_reports[0].provider_id,
        "heavy_body"
    );
    assert_eq!(
        output.family_request.parameters.get("radius"),
        Some(&shape_family_compile::FamilyValue::Scalar(0.2))
    );
    assert!(
        fixture
            .document
            .foundry_locks
            .iter()
            .any(|lock| lock.target == FoundryLockTarget::Control("radius".to_owned()))
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

struct RuntimeFixture {
    document: FoundryAssetDocument,
    catalog: TestCatalog,
    modern_style_ref: CatalogContentRef,
    modern_style_impl_ref: CatalogContentRef,
    incompatible_style_ref: CatalogContentRef,
    incompatible_style_impl_ref: CatalogContentRef,
}

impl RuntimeFixture {
    fn new() -> Self {
        let family = family_schema();
        let roman_style = style_kit("roman", "Roman", "bridge", &["roman_body", "heavy_body"]);
        let modern_style = style_kit("modern", "Modern", "bridge", &["modern_body"]);
        let incompatible_style = style_kit("alien", "Alien", "crate", &["alien_body"]);
        let family_impl = family_implementation();
        let roman_style_impl = style_implementation(
            "roman",
            "bridge",
            "roman_body",
            vec![
                provider_fragment("roman_body", 0.1),
                provider_fragment("heavy_body", 0.2),
            ],
        );
        let modern_style_impl = style_implementation(
            "modern",
            "bridge",
            "modern_body",
            vec![provider_fragment("modern_body", 0.12)],
        );
        let incompatible_style_impl = style_implementation(
            "alien",
            "crate",
            "alien_body",
            vec![provider_fragment("alien_body", 0.1)],
        );
        let profile = customizer_profile();

        let (family_ref, family_json) =
            catalog_entry("bridge-family", ASSET_FAMILY_SCHEMA_VERSION, &family);
        let (roman_style_ref, roman_style_json) =
            catalog_entry("roman-style", STYLE_KIT_SCHEMA_VERSION, &roman_style);
        let (modern_style_ref, modern_style_json) =
            catalog_entry("modern-style", STYLE_KIT_SCHEMA_VERSION, &modern_style);
        let (incompatible_style_ref, incompatible_style_json) =
            catalog_entry("alien-style", STYLE_KIT_SCHEMA_VERSION, &incompatible_style);
        let (family_impl_ref, family_impl_json) = catalog_entry(
            "bridge-family-impl",
            FAMILY_IMPLEMENTATION_SCHEMA_VERSION,
            &family_impl,
        );
        let (roman_style_impl_ref, roman_style_impl_json) = catalog_entry(
            "roman-style-impl",
            STYLE_IMPLEMENTATION_SCHEMA_VERSION,
            &roman_style_impl,
        );
        let (modern_style_impl_ref, modern_style_impl_json) = catalog_entry(
            "modern-style-impl",
            STYLE_IMPLEMENTATION_SCHEMA_VERSION,
            &modern_style_impl,
        );
        let (incompatible_style_impl_ref, incompatible_style_impl_json) = catalog_entry(
            "alien-style-impl",
            STYLE_IMPLEMENTATION_SCHEMA_VERSION,
            &incompatible_style_impl,
        );
        let (profile_ref, profile_json) = catalog_entry(
            "bridge-profile",
            shape_foundry::CUSTOMIZER_PROFILE_SCHEMA_VERSION,
            &profile,
        );

        let mut document = FoundryAssetDocument {
            schema_version: FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION,
            document_id: FoundryDocumentId("doc-runtime".to_owned()),
            family_content_ref: family_ref,
            style_content_ref: roman_style_ref,
            family_implementation_ref: family_impl_ref,
            style_implementation_ref: roman_style_impl_ref,
            customizer_profile_ref: profile_ref,
            control_state: BTreeMap::from([("radius".to_owned(), ControlValue::Scalar(0.15))]),
            provider_overrides: BTreeMap::new(),
            foundry_locks: Vec::new(),
            local_recipe_overrides: Vec::new(),
            seed: 42,
            catalog_lock: None,
            build_stamp: None,
        };
        document.catalog_lock = Some(FoundryCatalogLock {
            exact_refs: document_catalog_refs(&document),
            embedded_snapshots: Vec::new(),
            compiler_version: "0.1.0".to_owned(),
            catalog_version: 1,
        });

        let catalog = TestCatalog {
            entries: BTreeMap::from([
                ("bridge-family".to_owned(), family_json),
                ("roman-style".to_owned(), roman_style_json),
                ("modern-style".to_owned(), modern_style_json),
                ("alien-style".to_owned(), incompatible_style_json),
                ("bridge-family-impl".to_owned(), family_impl_json),
                ("roman-style-impl".to_owned(), roman_style_impl_json),
                ("modern-style-impl".to_owned(), modern_style_impl_json),
                ("alien-style-impl".to_owned(), incompatible_style_impl_json),
                ("bridge-profile".to_owned(), profile_json),
            ]),
        };

        Self {
            document,
            catalog,
            modern_style_ref,
            modern_style_impl_ref,
            incompatible_style_ref,
            incompatible_style_impl_ref,
        }
    }

    fn switch_to_modern_style(&mut self) {
        self.document.style_content_ref = self.modern_style_ref.clone();
        self.document.style_implementation_ref = self.modern_style_impl_ref.clone();
        self.document.catalog_lock = Some(FoundryCatalogLock {
            exact_refs: document_catalog_refs(&self.document),
            embedded_snapshots: Vec::new(),
            compiler_version: "0.1.0".to_owned(),
            catalog_version: 1,
        });
    }

    fn switch_to_incompatible_style(&mut self) {
        self.document.style_content_ref = self.incompatible_style_ref.clone();
        self.document.style_implementation_ref = self.incompatible_style_impl_ref.clone();
        self.document.catalog_lock = Some(FoundryCatalogLock {
            exact_refs: document_catalog_refs(&self.document),
            embedded_snapshots: Vec::new(),
            compiler_version: "0.1.0".to_owned(),
            catalog_version: 1,
        });
    }

    fn snapshots_for(&self, document: &FoundryAssetDocument) -> Vec<EmbeddedCatalogSnapshot> {
        document_catalog_refs(document)
            .into_values()
            .map(|content_ref| EmbeddedCatalogSnapshot {
                canonical_json: self.catalog.entries[&content_ref.stable_id].clone(),
                content_ref,
            })
            .collect()
    }
}

fn family_schema() -> AssetFamilySchema {
    AssetFamilySchema {
        schema_version: ASSET_FAMILY_SCHEMA_VERSION,
        id: "bridge".to_owned(),
        display_name: "Bridge".to_owned(),
        summary: "Runtime test bridge family".to_owned(),
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
        parameter_slots: vec![shape_family::FamilyParameterSlot {
            id: "radius".to_owned(),
            label: "Radius".to_owned(),
            target_role: Some("body".to_owned()),
            kind: FamilyParameterKind::Length {
                unit: LengthUnit::FamilyUnits,
            },
            range: Some(ParameterRange {
                minimum: 0.01,
                maximum: 0.5,
                step: 0.01,
            }),
            default_value: Some(FamilyDefaultValue::Scalar(0.1)),
            execution_policy: ParameterExecutionPolicy::RequiredBinding,
            topology_changing: false,
        }],
        constraints: Vec::new(),
        variant_rules: Vec::new(),
        export_requirements: Vec::new(),
        compatible_style_kits: vec!["roman".to_owned(), "modern".to_owned()],
        tags: Vec::new(),
    }
}

fn style_kit(id: &str, label: &str, family_id: &str, prototypes: &[&str]) -> StyleKit {
    StyleKit {
        schema_version: STYLE_KIT_SCHEMA_VERSION,
        id: id.to_owned(),
        display_name: label.to_owned(),
        compatible_families: vec![family_id.to_owned()],
        bevel_policy: BevelPolicy {
            width: LengthValue::FamilyUnits(0.05),
            segments: 2,
            profile: NormalizedBevelProfile { normalized: 0.5 },
        },
        profile_language: ProfileLanguage {
            curve_family: "rounded".to_owned(),
            allowed_profiles: vec!["soft".to_owned()],
            allow_asymmetry: false,
        },
        repetition: RepetitionPolicy {
            density: 0.5,
            preferred_spacing: LengthValue::FamilyUnits(1.0),
            maximum_default_count: 4,
        },
        symmetry: SymmetryPolicy {
            prefer_mirrors: false,
            allowed_axes: Vec::new(),
        },
        exaggeration: ExaggerationPolicy {
            silhouette: 0.0,
            detail: 0.0,
        },
        family_facets: BTreeMap::from([(
            family_id.to_owned(),
            FamilyStyleFacet {
                family_id: family_id.to_owned(),
                proportions: Vec::new(),
                part_prototypes: prototypes
                    .iter()
                    .map(|prototype| PartPrototype {
                        id: (*prototype).to_owned(),
                        display_name: (*prototype).to_owned(),
                        role: "body".to_owned(),
                        operation_tags: vec![AllowedOperationKind::Primitive],
                        style_tags: Vec::new(),
                    })
                    .collect(),
                detail_modules: Vec::new(),
                policy_overrides: FamilyStylePolicyOverrides::default(),
            },
        )]),
        tags: Vec::new(),
    }
}

fn family_implementation() -> FamilyImplementation {
    FamilyImplementation {
        schema_version: FAMILY_IMPLEMENTATION_SCHEMA_VERSION,
        family_id: "bridge".to_owned(),
        base_recipe: AssetRecipe::new(AssetId(1), "Base"),
        parameter_bindings: vec![ParameterBinding::Scalar {
            slot: "radius".to_owned(),
            role: "body".to_owned(),
            local_path: definition_scalar_path(PartDefinitionId(1), "geometry.rounded_box.radius"),
            transform: ScalarTransform::Direct,
        }],
        default_role_providers: BTreeMap::new(),
        fragments: BTreeMap::new(),
        attachment_bindings: Vec::new(),
    }
}

fn style_implementation(
    style_id: &str,
    family_id: &str,
    default_provider: &str,
    fragments: Vec<RecipeFragment>,
) -> StyleImplementation {
    StyleImplementation {
        schema_version: STYLE_IMPLEMENTATION_SCHEMA_VERSION,
        style_kit_id: style_id.to_owned(),
        family_id: family_id.to_owned(),
        default_role_providers: BTreeMap::from([("body".to_owned(), default_provider.to_owned())]),
        prototypes: fragments
            .into_iter()
            .map(|fragment| (fragment.id.clone(), fragment))
            .collect(),
        detail_modules: BTreeMap::new(),
    }
}

fn provider_fragment(id: &str, radius: f32) -> RecipeFragment {
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
        recipe: body_recipe(id, radius),
    }
}

fn body_recipe(title: &str, radius: f32) -> AssetRecipe {
    let definition_id = PartDefinitionId(1);
    let instance_id = PartInstanceId(1);
    let parameter_id = ParameterId(1);
    let definition = PartDefinition {
        id: definition_id,
        name: "Body".to_owned(),
        tags: BTreeSet::new(),
        geometry: GeometryRecipe {
            source: GeometrySource::RoundedBox {
                half_extents: [1.0, 0.5, 0.25],
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
    let descriptor = ParameterDescriptor {
        id: parameter_id,
        path: definition_scalar_path(definition_id, "geometry.rounded_box.radius"),
        label: "Radius".to_owned(),
        group: "Form".to_owned(),
        minimum: 0.0,
        maximum: 0.5,
        step: 0.01,
        mutation_sigma: 0.05,
        topology_changing: false,
        beginner_description: "Corner radius".to_owned(),
    };
    let mut recipe = AssetRecipe::new(AssetId(7), title);
    recipe.schema_version = ASSET_RECIPE_SCHEMA_VERSION;
    recipe.definitions.insert(definition_id, definition);
    recipe.instances.insert(instance_id, instance);
    recipe.root_instances.push(instance_id);
    recipe.parameters.insert(parameter_id, descriptor);
    recipe.next_ids.part_definition = 2;
    recipe.next_ids.part_instance = 2;
    recipe.next_ids.parameter = 2;
    recipe
}

fn customizer_profile() -> CustomizerProfile {
    let mut profile = CustomizerProfile::empty("bridge", Some("roman".to_owned()));
    profile.controls.push(CustomizerControl {
        id: "radius".to_owned(),
        label: "Radius".to_owned(),
        section: None,
        primary: true,
        visible: true,
        kind: ControlKind::ContinuousAxis { default: 0.1 },
        bindings: vec![ControlSlotBinding {
            slot: "radius".to_owned(),
            slot_policy: ParameterExecutionPolicy::RequiredBinding,
            response: ResponseCurve::Linear,
        }],
        domain: FeasibleControlDomain {
            continuous_intervals: vec![ClosedInterval {
                minimum: 0.01,
                maximum: 0.5,
            }],
            discrete_values: Vec::new(),
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::CertifiedContinuous,
        },
        topology_behavior: ControlTopologyBehavior::TopologyPreserving,
        divergence: ControlDivergence::Synced,
    });
    profile
}

fn radius_override(
    id: &str,
    base_geometry_fingerprint: shape_family_compile::identity::GeometryInputFingerprint,
    parameter: ParameterId,
    survival_policy: OverrideSurvivalPolicy,
    value: f32,
) -> LocalRecipeOverride {
    LocalRecipeOverride {
        id: LocalRecipeOverrideId(id.to_owned()),
        base_geometry_fingerprint,
        edit_program: AssetEditProgram {
            label: id.to_owned(),
            seed: 7,
            operations: vec![AssetEdit::SetScalar { parameter, value }],
        },
        touched_targets: vec![shape_foundry::TouchedSemanticTarget::Parameter(parameter)],
        survival_policy,
    }
}

fn provider_ref(stable_id: &str) -> CatalogContentRef {
    CatalogContentRef {
        stable_id: stable_id.to_owned(),
        schema_version: 1,
        fingerprint: CatalogContentFingerprint(ContentFingerprint([9; 32])),
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
