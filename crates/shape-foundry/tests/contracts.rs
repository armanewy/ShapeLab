use std::collections::BTreeMap;

use shape_family::ParameterExecutionPolicy;
use shape_family_compile::identity::{CatalogContentFingerprint, ContentFingerprint};
use shape_foundry::{
    CatalogContentRef, ChoiceOption, ClosedInterval, ControlDivergence, ControlKind,
    ControlSlotBinding, ControlTopologyBehavior, CustomizerControl, CustomizerProfile,
    DomainCertification, FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION, FeasibleControlDomain,
    FoundryAssetDocument, FoundryCommand, FoundryDocumentId, FoundryPackDocument,
    FoundryPackExportProfile, GenerateCandidatesRequest, PackCoherencePolicy, ResponseCurve,
    SURFACE_VISUAL_VARIATION_UNAVAILABLE_REASON, SharedProviderPolicy, VariationChannel,
    VariationIntent, VariationScope, WholeModelPreviewRef, apply_foundry_command,
    built_in_surface_capability_for_profile, parse_foundry_surface_capability_sidecar_json,
    validate_customizer_profile, validate_foundry_command, validate_foundry_document,
    validate_foundry_pack,
};

#[test]
fn exact_catalog_lock_mismatch_is_reported() {
    let mut document = document_fixture();
    let mut exact_refs = BTreeMap::new();
    exact_refs.insert("family".to_owned(), content_ref("wrong-family", 9));
    exact_refs.insert("style".to_owned(), document.style_content_ref.clone());
    exact_refs.insert(
        "family_impl".to_owned(),
        document.family_implementation_ref.clone(),
    );
    exact_refs.insert(
        "style_impl".to_owned(),
        document.style_implementation_ref.clone(),
    );
    exact_refs.insert(
        "customizer_profile".to_owned(),
        document.customizer_profile_ref.clone(),
    );
    document.catalog_lock = Some(shape_foundry::FoundryCatalogLock {
        exact_refs,
        embedded_snapshots: Vec::new(),
        compiler_version: "0.1.0".to_owned(),
        catalog_version: 1,
    });

    let report = validate_foundry_document(&document);

    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "catalog_lock_ref_mismatch")
    );
}

#[test]
fn control_validation_rejects_conflicts_and_uncertified_sliders() {
    let mut profile = CustomizerProfile::empty("bridge", Some("roman".to_owned()));
    profile.controls = vec![
        slider_control("heft", "Structural Heft", "span_length"),
        slider_control("heft_duplicate", "Duplicate Heft", "span_length"),
    ];
    profile.controls[0].domain.certification = DomainCertification::Uncertified {
        reason: "sample-only".to_owned(),
    };

    let report = validate_customizer_profile(&profile);

    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "conflicting_control_ownership")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "uncertified_continuous_control")
    );
}

#[test]
fn discrete_gallery_with_whole_model_previews_is_valid() {
    let mut profile = CustomizerProfile::empty("bridge", Some("roman".to_owned()));
    profile.controls.push(CustomizerControl {
        id: "support_count".to_owned(),
        label: "Support Count".to_owned(),
        section: None,
        primary: true,
        visible: true,
        kind: ControlKind::ChoiceGallery {
            options: vec![
                option("two", "Two Supports"),
                option("four", "Four Supports"),
            ],
        },
        bindings: vec![ControlSlotBinding {
            slot: "support_count".to_owned(),
            slot_policy: ParameterExecutionPolicy::RequiredBinding,
            response: ResponseCurve::Linear,
        }],
        domain: FeasibleControlDomain {
            continuous_intervals: Vec::new(),
            discrete_values: vec![
                shape_foundry::ControlValue::Choice("two".to_owned()),
                shape_foundry::ControlValue::Choice("four".to_owned()),
            ],
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::DiscreteSamples,
        },
        topology_behavior: ControlTopologyBehavior::TopologyChanging,
        divergence: ControlDivergence::Synced,
    });

    assert!(validate_customizer_profile(&profile).is_valid());
}

#[test]
fn command_validation_checks_profile_references() {
    let profile = CustomizerProfile::empty("crate", None);
    let command = FoundryCommand::GenerateCandidates(GenerateCandidatesRequest {
        strategy_id: Some("missing".to_owned()),
        count: 0,
        seed: 7,
        variation_intent: VariationIntent::default(),
    });

    let report = validate_foundry_command(&command, None, Some(&profile));

    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "empty_candidate_request")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "unknown_candidate_strategy")
    );
}

#[test]
fn variation_scope_and_channel_round_trip_through_serde() {
    let scope = VariationScope::SemanticPartGroup {
        group_id: "edge-trim".to_owned(),
        display_name: "Edge Trim".to_owned(),
    };
    let channel = VariationChannel::Custom {
        channel_id: "future-material-pass".to_owned(),
        display_name: "Future Material Pass".to_owned(),
    };

    assert_eq!(
        serde_json::from_str::<VariationScope>(&serde_json::to_string(&scope).unwrap()).unwrap(),
        scope
    );
    assert_eq!(
        serde_json::from_str::<VariationChannel>(&serde_json::to_string(&channel).unwrap())
            .unwrap(),
        channel
    );
}

#[test]
fn default_variation_intent_is_whole_asset_complete_look() {
    let intent = VariationIntent::default();

    assert_eq!(intent.scope, VariationScope::WholeAsset);
    assert_eq!(intent.channels, vec![VariationChannel::CompleteLook]);
}

#[test]
fn existing_documents_deserialize_with_default_variation_state() {
    let mut json = serde_json::to_value(document_fixture()).unwrap();
    json.as_object_mut()
        .expect("document fixture is an object")
        .remove("variation_state");

    let document: FoundryAssetDocument = serde_json::from_value(json).unwrap();

    assert_eq!(
        document.variation_state.intent,
        VariationIntent::complete_look()
    );
}

#[test]
fn variation_focus_commands_replay_deterministically() {
    let mut first = document_fixture();
    let mut second = document_fixture();
    let commands = [
        FoundryCommand::SetVariationIntent {
            intent: VariationIntent::whole_asset_shape(),
        },
        FoundryCommand::SetFocusPartGroup {
            group_id: "body".to_owned(),
        },
        FoundryCommand::ClearVariationFocus,
    ];

    for command in &commands {
        apply_foundry_command(&mut first, command).unwrap();
        apply_foundry_command(&mut second, command).unwrap();
    }

    assert_eq!(first.variation_state, second.variation_state);
    assert_eq!(
        first.variation_state.intent.scope,
        VariationScope::WholeAsset
    );
    assert_eq!(
        first.variation_state.intent.channels,
        vec![VariationChannel::Shape]
    );
}

#[test]
fn sci_fi_crate_reports_surface_package_without_enabling_surface_candidates() {
    let capability = built_in_surface_capability_for_profile("sci-fi-crate-profile");

    assert!(capability.surface_package_available);
    assert!(capability.surface_payload_ready);
    assert!(capability.uv_ready);
    assert!(capability.surface_visual_evidence_ready);
    assert_eq!(capability.material_slot_count, 6);
    assert_eq!(
        capability.texture_channels,
        vec!["Base color", "Metallic roughness", "Normal", "Occlusion"]
    );
    assert!(!capability.visual_surface_variation_ready);
    assert!(!capability.surface_candidate_mode_available());
    assert!(!capability.focus_part_surface_ready);
    assert_eq!(
        capability.surface_mode_unavailable_reason(),
        SURFACE_VISUAL_VARIATION_UNAVAILABLE_REASON
    );
}

#[test]
fn surface_capability_sidecar_maps_to_product_view_without_ui_overclaim() {
    let sidecar = r#"{
        "schema_version": 1,
        "profile_id": "sci-fi-crate",
        "surface_payload_ready": true,
        "uv_ready": true,
        "material_slots": [
            "painted_metal_body",
            "dark_rubber_grips"
        ],
        "texture_channels": [
            "base_color",
            "metallic_roughness",
            "normal",
            "occlusion"
        ],
        "variation_channels_supported": {
            "surface": true,
            "wear": false
        },
        "surface_visual_evidence_ready": true,
        "focus_part_surface_ready": false,
        "human_label": "Sci-Fi Crate Surface Payload",
        "unavailable_reasons": [
            "Wear variation is metadata-only in Surface Lab v1."
        ]
    }"#;

    let capability =
        parse_foundry_surface_capability_sidecar_json(sidecar).expect("sidecar should parse");

    assert!(capability.surface_package_available);
    assert!(capability.surface_payload_ready);
    assert!(capability.uv_ready);
    assert!(capability.surface_visual_evidence_ready);
    assert_eq!(capability.material_slot_count, 2);
    assert_eq!(
        capability.texture_channels,
        vec!["Base color", "Metallic roughness", "Normal", "Occlusion"]
    );
    assert!(!capability.visual_surface_variation_ready);
    assert!(!capability.focus_part_surface_ready);
    assert!(
        capability
            .unavailable_reasons
            .contains(&SURFACE_VISUAL_VARIATION_UNAVAILABLE_REASON.to_owned())
    );
}

#[test]
fn surface_capability_visual_evidence_does_not_enable_app_surface_modes() {
    let sidecar = r#"{
        "schema_version": 1,
        "profile_id": "sci-fi-crate",
        "surface_payload_ready": true,
        "uv_ready": true,
        "material_slots": [
            "painted_metal_body"
        ],
        "texture_channels": [
            "base_color"
        ],
        "variation_channels_supported": {
            "surface": true,
            "wear": false
        },
        "surface_visual_evidence_ready": true,
        "focus_part_surface_ready": true,
        "human_label": "Sci-Fi Crate Surface Payload",
        "unavailable_reasons": []
    }"#;

    let capability =
        parse_foundry_surface_capability_sidecar_json(sidecar).expect("sidecar should parse");

    assert!(capability.surface_package_available);
    assert!(capability.surface_payload_ready);
    assert!(capability.surface_visual_evidence_ready);
    assert!(!capability.visual_surface_variation_ready);
    assert!(!capability.surface_candidate_mode_available());
    assert!(!capability.focus_part_surface_ready);
    assert!(
        capability
            .unavailable_reasons
            .contains(&SURFACE_VISUAL_VARIATION_UNAVAILABLE_REASON.to_owned())
    );
}

#[test]
fn surface_capability_sidecar_rejects_malformed_or_local_path_data() {
    let malformed = r#"{
        "schema_version": 1,
        "profile_id": "sci-fi-crate",
        "surface_payload_ready": true,
        "uv_ready": false,
        "material_slots": [],
        "texture_channels": [],
        "variation_channels_supported": ["surface"],
        "focus_part_surface_ready": false,
        "human_label": "Sci-Fi Crate Surface Payload",
        "unavailable_reasons": []
    }"#;
    let error = parse_foundry_surface_capability_sidecar_json(malformed)
        .expect_err("ready payload without evidence should fail");
    assert!(
        error
            .diagnostic()
            .contains("cannot mark payload ready without UV")
    );

    let absolute_path = r#"{
        "schema_version": 1,
        "profile_id": "sci-fi-crate",
        "surface_payload_ready": false,
        "uv_ready": false,
        "material_slots": [],
        "texture_channels": [],
        "variation_channels_supported": ["surface"],
        "focus_part_surface_ready": false,
        "human_label": "C:\\Users\\artist\\surface-capabilities.json",
        "unavailable_reasons": []
    }"#;
    let error = parse_foundry_surface_capability_sidecar_json(absolute_path)
        .expect_err("absolute local path should fail");
    assert!(error.diagnostic().contains("absolute local paths"));

    let visual_without_payload = r#"{
        "schema_version": 1,
        "profile_id": "sci-fi-crate",
        "surface_payload_ready": false,
        "uv_ready": true,
        "material_slots": ["painted_metal_body"],
        "texture_channels": ["base_color"],
        "variation_channels_supported": ["surface"],
        "surface_visual_evidence_ready": true,
        "focus_part_surface_ready": false,
        "human_label": "Sci-Fi Crate Surface Payload",
        "unavailable_reasons": []
    }"#;
    let error = parse_foundry_surface_capability_sidecar_json(visual_without_payload)
        .expect_err("visual evidence without payload should fail");
    assert!(
        error
            .diagnostic()
            .contains("cannot mark visual evidence ready without a surface payload")
    );
}

#[test]
fn set_control_validation_checks_kind_and_feasible_domain() {
    let mut profile = CustomizerProfile::empty("bridge", Some("roman".to_owned()));
    profile
        .controls
        .push(slider_control("span_length", "Span Length", "span_length"));

    let wrong_kind = FoundryCommand::SetControl {
        control_id: "span_length".to_owned(),
        value: shape_foundry::ControlValue::Choice("wide".to_owned()),
    };
    let outside_domain = FoundryCommand::SetControl {
        control_id: "span_length".to_owned(),
        value: shape_foundry::ControlValue::Scalar(8.0),
    };

    assert!(
        validate_foundry_command(&wrong_kind, None, Some(&profile))
            .issues
            .iter()
            .any(|issue| issue.code == "control_value_kind_mismatch")
    );
    assert!(
        validate_foundry_command(&outside_domain, None, Some(&profile))
            .issues
            .iter()
            .any(|issue| issue.code == "control_value_outside_domain")
    );
}

#[test]
fn provider_gallery_round_trips_through_set_control() {
    let mut profile = CustomizerProfile::empty("bridge", Some("roman".to_owned()));
    profile.controls.push(provider_control());
    let command = FoundryCommand::SetControl {
        control_id: "support_provider".to_owned(),
        value: shape_foundry::ControlValue::Provider("timber_support".to_owned()),
    };

    assert!(validate_foundry_command(&command, None, Some(&profile)).is_valid());
}

#[test]
fn exact_family_style_pack_rejects_incompatible_member() {
    let shared_family = content_ref("bridge-family", 1);
    let shared_style = content_ref("roman-style", 2);
    let mut pack = FoundryPackDocument::new(
        "bridge-pack",
        shared_family,
        shared_style,
        FoundryPackExportProfile {
            profile: "game-runtime".to_owned(),
            require_all_members: true,
        },
    );
    pack.coherence_policy = PackCoherencePolicy::ExactFamilyAndStyle;
    let mut incompatible = document_fixture_with_style("scifi-style");
    incompatible.family_content_ref = content_ref("crate-family", 8);
    pack.members.insert("crate".to_owned(), incompatible);

    let report = validate_foundry_pack(&pack);

    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "pack_member_family_mismatch")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "pack_member_style_mismatch")
    );
}

#[test]
fn pack_validation_rejects_shared_provider_conflicts_and_lock_mismatch() {
    let shared_family = content_ref("bridge-family", 1);
    let shared_style = content_ref("roman-style", 2);
    let shared_provider = content_ref("timber-support", 7);
    let mut pack = FoundryPackDocument::new(
        "bridge-pack",
        shared_family,
        shared_style,
        FoundryPackExportProfile {
            profile: "game-runtime".to_owned(),
            require_all_members: true,
        },
    );
    pack.shared_provider_policy = SharedProviderPolicy::SharedExact(BTreeMap::from([(
        "support".to_owned(),
        shared_provider,
    )]));
    pack.catalog_lock = Some(shape_foundry::FoundryCatalogLock {
        exact_refs: BTreeMap::from([("family".to_owned(), content_ref("wrong-family", 9))]),
        embedded_snapshots: Vec::new(),
        compiler_version: "0.1.0".to_owned(),
        catalog_version: 1,
    });
    let mut member = document_fixture();
    member.provider_overrides.insert(
        "support".to_owned(),
        shape_foundry::ProviderOverride {
            role: "support".to_owned(),
            provider_ref: content_ref("stone-support", 10),
        },
    );
    pack.members.insert("member".to_owned(), member);

    let report = validate_foundry_pack(&pack);

    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "catalog_lock_ref_mismatch")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "pack_member_provider_conflict")
    );
}

fn slider_control(id: &str, label: &str, slot: &str) -> CustomizerControl {
    CustomizerControl {
        id: id.to_owned(),
        label: label.to_owned(),
        section: None,
        primary: true,
        visible: true,
        kind: ControlKind::ContinuousAxis { default: 0.0 },
        bindings: vec![ControlSlotBinding {
            slot: slot.to_owned(),
            slot_policy: ParameterExecutionPolicy::RequiredBinding,
            response: ResponseCurve::Piecewise {
                points: vec![[-1.0, 0.5], [1.0, 1.5]],
                monotonic: true,
            },
        }],
        domain: FeasibleControlDomain {
            continuous_intervals: vec![ClosedInterval {
                minimum: -1.0,
                maximum: 1.0,
            }],
            discrete_values: Vec::new(),
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::CertifiedContinuous,
        },
        topology_behavior: ControlTopologyBehavior::TopologyPreserving,
        divergence: ControlDivergence::Synced,
    }
}

fn option(value: &str, label: &str) -> ChoiceOption {
    ChoiceOption {
        value: value.to_owned(),
        label: label.to_owned(),
        preview: WholeModelPreviewRef {
            preview_id: format!("preview-{value}"),
            artifact_fingerprint: None,
        },
    }
}

fn provider_control() -> CustomizerControl {
    CustomizerControl {
        id: "support_provider".to_owned(),
        label: "Support Provider".to_owned(),
        section: None,
        primary: true,
        visible: true,
        kind: ControlKind::ProviderGallery {
            role: "support".to_owned(),
            options: vec![shape_foundry::ProviderOption {
                provider_id: "timber_support".to_owned(),
                label: "Timber Support".to_owned(),
                preview: WholeModelPreviewRef {
                    preview_id: "preview-timber-support".to_owned(),
                    artifact_fingerprint: None,
                },
            }],
        },
        bindings: vec![ControlSlotBinding {
            slot: "support_provider".to_owned(),
            slot_policy: ParameterExecutionPolicy::RequiredBinding,
            response: ResponseCurve::Linear,
        }],
        domain: FeasibleControlDomain {
            continuous_intervals: Vec::new(),
            discrete_values: vec![shape_foundry::ControlValue::Provider(
                "timber_support".to_owned(),
            )],
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::DiscreteSamples,
        },
        topology_behavior: ControlTopologyBehavior::TopologyChanging,
        divergence: ControlDivergence::Synced,
    }
}

fn document_fixture() -> FoundryAssetDocument {
    document_fixture_with_style("roman-style")
}

fn document_fixture_with_style(style_id: &str) -> FoundryAssetDocument {
    FoundryAssetDocument {
        schema_version: FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION,
        document_id: FoundryDocumentId("doc-1".to_owned()),
        family_content_ref: content_ref("bridge-family", 1),
        style_content_ref: content_ref(style_id, 2),
        family_implementation_ref: content_ref("bridge-family-impl", 3),
        style_implementation_ref: content_ref("roman-style-impl", 4),
        customizer_profile_ref: content_ref("bridge-profile", 5),
        control_state: BTreeMap::from([(
            "span_length".to_owned(),
            shape_foundry::ControlValue::Scalar(3.0),
        )]),
        provider_overrides: BTreeMap::new(),
        foundry_locks: Vec::new(),
        variation_state: shape_foundry::FoundryVariationState::default(),
        local_recipe_overrides: Vec::new(),
        seed: 11,
        catalog_lock: None,
        build_stamp: None,
    }
}

fn content_ref(stable_id: &str, byte: u8) -> CatalogContentRef {
    CatalogContentRef {
        stable_id: stable_id.to_owned(),
        schema_version: 1,
        fingerprint: CatalogContentFingerprint(ContentFingerprint([byte; 32])),
    }
}
