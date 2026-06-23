#![forbid(unsafe_code)]
#![allow(dead_code)]

#[path = "../src/foundry/mod.rs"]
mod foundry;

use std::collections::BTreeMap;

use foundry::{
    FoundryAppCommand,
    panels::customize::{
        CustomizeDeckOptions, advanced_recipe_open_command, advanced_recipe_rows,
        choose_option_intents, control_can_reset, control_lock_command, customize_deck,
        preview_control_value_intents, release_control_value_intents, reset_control_intents,
        select_control_command,
    },
    view_model::FoundryControlPresentation,
};
use shape_foundry::{
    CatalogContentRef, ChoiceOption, ClosedInterval, ControlDivergence, ControlEvaluationContext,
    ControlKind, ControlTopologyBehavior, ControlValue, CustomizerControl, CustomizerProfile,
    DomainCertification, FeasibleControlDomain, FoundryAssetDocument, FoundryCommand,
    FoundryDocumentId, FoundryLock, FoundryLockMode, FoundryLockTarget, ProviderOption,
    WholeModelPreviewRef,
};

#[test]
fn deck_defaults_to_primary_controls_and_collapsed_advanced() {
    let profile = deck_profile();
    let mut document = document_fixture();
    document
        .control_state
        .insert("height".to_owned(), ControlValue::Scalar(0.5));
    document.foundry_locks.push(FoundryLock {
        target: FoundryLockTarget::Control("count".to_owned()),
        mode: FoundryLockMode::Locked,
        reason: Some("fixed".to_owned()),
    });
    let context = ControlEvaluationContext::new(&[]);

    let deck = customize_deck(
        &profile,
        &document,
        context,
        CustomizeDeckOptions {
            preview_sample_count: 3,
            ..CustomizeDeckOptions::default()
        },
    )
    .expect("deck should build");

    assert!(!deck.advanced_open);
    assert_eq!(
        control_ids(&deck.controls),
        vec!["height", "count", "detail", "finish", "support_provider"]
    );
    assert_eq!(
        control_ids(&deck.primary_controls),
        vec!["height", "count", "finish", "support_provider"]
    );
    assert_eq!(control_ids(&deck.advanced_controls), vec!["detail"]);
    assert_eq!(
        control_ids(&deck.displayed_controls),
        vec!["height", "count", "finish", "support_provider"]
    );

    let height = &deck.primary_controls[0];
    assert_eq!(
        height.presentation,
        FoundryControlPresentation::ContinuousMacroAxis
    );
    assert_eq!(height.kind, "Macro Axis");
    assert_eq!(height.value, Some(ControlValue::Scalar(0.5)));
    assert_eq!(height.default_value, Some(ControlValue::Scalar(0.0)));
    assert!(!height.locked);
    assert!(control_can_reset(height));

    let count = &deck.primary_controls[1];
    assert_eq!(count.presentation, FoundryControlPresentation::Stepper);
    assert!(count.locked);
    assert!(!control_can_reset(count));
    assert_eq!(
        advanced_recipe_open_command(true),
        FoundryAppCommand::SetAdvancedRecipeOpen(true)
    );
    assert_eq!(
        select_control_command(Some("height".to_owned())),
        FoundryAppCommand::SelectControl(Some("height".to_owned()))
    );
}

#[test]
fn opening_advanced_adds_non_primary_controls_without_hidden_rows() {
    let profile = deck_profile();
    let document = document_fixture();

    let deck = customize_deck(
        &profile,
        &document,
        ControlEvaluationContext::new(&[]),
        CustomizeDeckOptions {
            advanced_open: true,
            preview_sample_count: 3,
        },
    )
    .expect("deck should build");

    assert!(deck.advanced_open);
    assert_eq!(
        control_ids(&deck.displayed_controls),
        vec!["height", "count", "finish", "support_provider", "detail"]
    );
    assert!(
        deck.controls
            .iter()
            .all(|control| control.id != "debug_path")
    );
}

#[test]
fn filmstrip_and_gallery_options_are_deterministic() {
    let profile = deck_profile();
    let mut document = document_fixture();
    document.control_state.insert(
        "finish".to_owned(),
        ControlValue::Choice("smooth".to_owned()),
    );
    document.control_state.insert(
        "support_provider".to_owned(),
        ControlValue::Provider("timber".to_owned()),
    );

    let deck = customize_deck(
        &profile,
        &document,
        ControlEvaluationContext::new(&[]),
        CustomizeDeckOptions {
            preview_sample_count: 3,
            ..CustomizeDeckOptions::default()
        },
    )
    .expect("deck should build");

    let height = control(&deck.controls, "height");
    assert_eq!(
        height
            .options
            .iter()
            .map(|option| option.value.clone())
            .collect::<Vec<_>>(),
        vec![
            ControlValue::Scalar(-1.0),
            ControlValue::Scalar(0.0),
            ControlValue::Scalar(1.0)
        ]
    );
    assert_eq!(
        height
            .options
            .iter()
            .map(|option| option.preview_id.as_deref())
            .collect::<Vec<_>>(),
        vec![
            Some("height-preview-0"),
            Some("height-preview-1"),
            Some("height-preview-2")
        ]
    );

    let enabled = control(&deck.controls, "detail");
    assert_eq!(enabled.presentation, FoundryControlPresentation::Toggle);
    assert_eq!(
        enabled
            .options
            .iter()
            .map(|option| option.label.as_str())
            .collect::<Vec<_>>(),
        vec!["Off", "On"]
    );

    let finish = control(&deck.controls, "finish");
    assert_eq!(
        finish.presentation,
        FoundryControlPresentation::ChoiceGallery
    );
    assert_eq!(
        finish
            .options
            .iter()
            .map(|option| (
                option.label.as_str(),
                option.preview_id.as_deref(),
                option.selected,
                option.unavailable_reason.as_deref(),
            ))
            .collect::<Vec<_>>(),
        vec![
            ("Smooth", Some("preview-smooth"), true, None),
            (
                "Ribbed",
                Some("preview-ribbed"),
                false,
                Some("requires supports")
            ),
        ]
    );

    let provider = control(&deck.controls, "support_provider");
    assert_eq!(
        provider.presentation,
        FoundryControlPresentation::ProviderGallery
    );
    assert_eq!(
        provider
            .options
            .iter()
            .map(|option| (
                option.provider_role.as_deref(),
                option.preview_id.as_deref(),
                option.selected,
            ))
            .collect::<Vec<_>>(),
        vec![
            (Some("support"), Some("preview-timber"), true),
            (Some("support"), Some("preview-stone"), false),
        ]
    );
}

#[test]
fn command_intents_wrap_generic_foundry_commands_and_exact_release_builds() {
    let profile = deck_profile();
    let mut document = document_fixture();
    document
        .control_state
        .insert("height".to_owned(), ControlValue::Scalar(0.5));
    let deck = customize_deck(
        &profile,
        &document,
        ControlEvaluationContext::new(&[]),
        CustomizeDeckOptions::default(),
    )
    .expect("deck should build");
    let height = control(&deck.controls, "height");

    let preview = preview_control_value_intents(height, ControlValue::Scalar(0.25));
    assert_eq!(preview.len(), 2);
    assert_eq!(preview[1], FoundryAppCommand::RequestPreview);
    assert_eq!(
        preview[0].single_foundry_command(),
        Some(&FoundryCommand::SetControl {
            control_id: "height".to_owned(),
            value: ControlValue::Scalar(0.25)
        })
    );

    let release = release_control_value_intents(height, ControlValue::Scalar(0.75));
    assert_eq!(release.len(), 2);
    assert_eq!(release[1], FoundryAppCommand::RequestBuild);
    assert_eq!(
        release[0].single_foundry_command(),
        Some(&FoundryCommand::SetControl {
            control_id: "height".to_owned(),
            value: ControlValue::Scalar(0.75)
        })
    );

    let reset = reset_control_intents(height);
    assert_eq!(
        reset[0].single_foundry_command(),
        Some(&FoundryCommand::ResetControl {
            control_id: "height".to_owned()
        })
    );
    assert_eq!(reset[1], FoundryAppCommand::RequestBuild);

    let finish = control(&deck.controls, "finish");
    let unavailable = &finish.options[1];
    assert!(choose_option_intents(finish, unavailable).is_empty());
    let available = &finish.options[0];
    assert_eq!(
        choose_option_intents(finish, available)[0].single_foundry_command(),
        Some(&FoundryCommand::SetControl {
            control_id: "finish".to_owned(),
            value: ControlValue::Choice("smooth".to_owned())
        })
    );
}

#[test]
fn lock_commands_avoid_duplicates_and_downgrade_to_search_protected() {
    let profile = deck_profile();
    let mut document = document_fixture();
    document.foundry_locks.push(FoundryLock {
        target: FoundryLockTarget::Control("height".to_owned()),
        mode: FoundryLockMode::Locked,
        reason: None,
    });
    let deck = customize_deck(
        &profile,
        &document,
        ControlEvaluationContext::new(&[]),
        CustomizeDeckOptions::default(),
    )
    .expect("deck should build");
    let locked_height = control(&deck.controls, "height");
    assert!(control_lock_command(locked_height, true).is_none());

    let unlock = control_lock_command(locked_height, false).expect("unlock intent");
    assert_eq!(
        unlock.single_foundry_command(),
        Some(&FoundryCommand::SetLock {
            lock: FoundryLock {
                target: FoundryLockTarget::Control("height".to_owned()),
                mode: FoundryLockMode::SearchProtected,
                reason: None,
            }
        })
    );

    let unlocked_count = control(&deck.controls, "count");
    let lock = control_lock_command(unlocked_count, true).expect("lock intent");
    assert_eq!(
        lock.single_foundry_command(),
        Some(&FoundryCommand::SetLock {
            lock: FoundryLock {
                target: FoundryLockTarget::Control("count".to_owned()),
                mode: FoundryLockMode::Locked,
                reason: Some("Locked from customizer deck".to_owned()),
            }
        })
    );
}

#[test]
fn divergence_and_technical_paths_stay_in_advanced_recipe_rows() {
    let profile = deck_profile();
    let document = document_fixture();
    let deck = customize_deck(
        &profile,
        &document,
        ControlEvaluationContext::new(&[]),
        CustomizeDeckOptions::default(),
    )
    .expect("deck should build");
    let height = control(&deck.controls, "height");

    assert_eq!(height.divergence, ControlDivergence::ConstraintLimited);
    assert_eq!(height.label, "Height");
    assert_eq!(height.kind, "Macro Axis");
    assert_eq!(height.help, None);
    assert!(
        height.advanced_path.as_deref().is_some_and(|path| {
            path == "controls.height" || path.starts_with("controls.height.")
        })
    );
    assert!(!height.label.contains("controls."));
    assert!(!height.kind.contains("controls."));

    let rows = advanced_recipe_rows(&deck);
    let height_row = rows
        .iter()
        .find(|row| row.control_id == "height")
        .expect("advanced height row");
    assert_eq!(height_row.technical_path, "controls.height");
    assert_eq!(height_row.divergence, "Constraint limited");
}

fn control_ids(controls: &[foundry::FoundryControlView]) -> Vec<&str> {
    controls.iter().map(|control| control.id.as_str()).collect()
}

fn control<'a>(
    controls: &'a [foundry::FoundryControlView],
    id: &str,
) -> &'a foundry::FoundryControlView {
    controls
        .iter()
        .find(|control| control.id == id)
        .unwrap_or_else(|| panic!("missing control {id}"))
}

fn deck_profile() -> CustomizerProfile {
    let mut profile = CustomizerProfile::empty("shape", None);
    profile.controls = vec![
        continuous_axis("height", "Height", true, true),
        stepper("count", "Count", true, true),
        toggle("detail", "Detail", false, true),
        choice_gallery("finish", "Finish", true, true),
        provider_gallery("support_provider", "Support Provider", true, true),
        toggle("debug_path", "Debug Path", true, false),
    ];
    profile
}

fn continuous_axis(id: &str, label: &str, primary: bool, visible: bool) -> CustomizerControl {
    CustomizerControl {
        id: id.to_owned(),
        label: label.to_owned(),
        section: None,
        primary,
        visible,
        kind: ControlKind::ContinuousAxis { default: 0.0 },
        bindings: Vec::new(),
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
        divergence: ControlDivergence::ConstraintLimited,
    }
}

fn stepper(id: &str, label: &str, primary: bool, visible: bool) -> CustomizerControl {
    CustomizerControl {
        id: id.to_owned(),
        label: label.to_owned(),
        section: None,
        primary,
        visible,
        kind: ControlKind::IntegerStepper { default: 2 },
        bindings: Vec::new(),
        domain: FeasibleControlDomain {
            continuous_intervals: Vec::new(),
            discrete_values: vec![
                ControlValue::Integer(1),
                ControlValue::Integer(2),
                ControlValue::Integer(3),
            ],
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::DiscreteSamples,
        },
        topology_behavior: ControlTopologyBehavior::TopologyChanging,
        divergence: ControlDivergence::Synced,
    }
}

fn toggle(id: &str, label: &str, primary: bool, visible: bool) -> CustomizerControl {
    CustomizerControl {
        id: id.to_owned(),
        label: label.to_owned(),
        section: None,
        primary,
        visible,
        kind: ControlKind::Toggle { default: false },
        bindings: Vec::new(),
        domain: FeasibleControlDomain {
            continuous_intervals: Vec::new(),
            discrete_values: vec![ControlValue::Toggle(false), ControlValue::Toggle(true)],
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::DiscreteSamples,
        },
        topology_behavior: ControlTopologyBehavior::TopologyPreserving,
        divergence: ControlDivergence::Synced,
    }
}

fn choice_gallery(id: &str, label: &str, primary: bool, visible: bool) -> CustomizerControl {
    CustomizerControl {
        id: id.to_owned(),
        label: label.to_owned(),
        section: None,
        primary,
        visible,
        kind: ControlKind::ChoiceGallery {
            options: vec![
                choice_option("smooth", "Smooth"),
                choice_option("ribbed", "Ribbed"),
            ],
        },
        bindings: Vec::new(),
        domain: FeasibleControlDomain {
            continuous_intervals: Vec::new(),
            discrete_values: vec![
                ControlValue::Choice("smooth".to_owned()),
                ControlValue::Choice("ribbed".to_owned()),
            ],
            unavailable_options: BTreeMap::from([(
                "ribbed".to_owned(),
                "requires supports".to_owned(),
            )]),
            certification: DomainCertification::DiscreteSamples,
        },
        topology_behavior: ControlTopologyBehavior::TopologyChanging,
        divergence: ControlDivergence::Synced,
    }
}

fn provider_gallery(id: &str, label: &str, primary: bool, visible: bool) -> CustomizerControl {
    CustomizerControl {
        id: id.to_owned(),
        label: label.to_owned(),
        section: None,
        primary,
        visible,
        kind: ControlKind::ProviderGallery {
            role: "support".to_owned(),
            options: vec![
                provider_option("timber", "Timber"),
                provider_option("stone", "Stone"),
            ],
        },
        bindings: Vec::new(),
        domain: FeasibleControlDomain {
            continuous_intervals: Vec::new(),
            discrete_values: vec![
                ControlValue::Provider("timber".to_owned()),
                ControlValue::Provider("stone".to_owned()),
            ],
            unavailable_options: BTreeMap::new(),
            certification: DomainCertification::DiscreteSamples,
        },
        topology_behavior: ControlTopologyBehavior::TopologyChanging,
        divergence: ControlDivergence::Synced,
    }
}

fn choice_option(value: &str, label: &str) -> ChoiceOption {
    ChoiceOption {
        value: value.to_owned(),
        label: label.to_owned(),
        preview: preview(value),
    }
}

fn provider_option(value: &str, label: &str) -> ProviderOption {
    ProviderOption {
        provider_id: value.to_owned(),
        label: label.to_owned(),
        preview: preview(value),
    }
}

fn preview(value: &str) -> WholeModelPreviewRef {
    WholeModelPreviewRef {
        preview_id: format!("preview-{value}"),
        artifact_fingerprint: None,
    }
}

fn document_fixture() -> FoundryAssetDocument {
    FoundryAssetDocument::new(
        FoundryDocumentId("doc".to_owned()),
        content_ref("family", 1),
        content_ref("style", 2),
        content_ref("family_impl", 3),
        content_ref("style_impl", 4),
        content_ref("profile", 5),
    )
}

fn content_ref(stable_id: &str, byte: u8) -> CatalogContentRef {
    let fingerprint = format!("{byte:02x}").repeat(32);
    serde_json::from_value(serde_json::json!({
        "stable_id": stable_id,
        "schema_version": 1,
        "fingerprint": fingerprint,
    }))
    .expect("test catalog reference is valid")
}
