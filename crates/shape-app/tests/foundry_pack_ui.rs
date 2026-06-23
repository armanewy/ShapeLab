#![forbid(unsafe_code)]
#![allow(dead_code)]

#[path = "../src/foundry/mod.rs"]
mod foundry;

use std::collections::BTreeMap;

use foundry::{
    FoundryJobRequest,
    panels::pack::{
        PackMemberStatus, add_current_asset_to_pack_command, batch_export_compile_request,
        batch_validate_pack, pack_contact_sheet, pack_panel_view, pack_view_from_document,
    },
};
use shape_foundry::{
    CatalogContentRef, ControlValue, FoundryAssetDocument, FoundryCommand, FoundryDocumentId,
    FoundryLock, FoundryLockMode, FoundryLockTarget, FoundryPackDocument, FoundryPackExportProfile,
    ProviderOverride, SharedProviderPolicy,
};

#[test]
fn add_current_asset_uses_foundry_pack_command() {
    let command = add_current_asset_to_pack_command("roman_bridge_pack", "wide_span");

    assert!(matches!(
        command.single_foundry_command(),
        Some(FoundryCommand::AddCurrentToPack { pack_id, member_id })
            if pack_id == "roman_bridge_pack" && member_id == "wide_span"
    ));
}

#[test]
fn pack_panel_exposes_members_shared_state_overrides_and_actions() {
    let view = pack_view_from_document(fixture_pack(), Some("tall_variant".to_owned()));
    let panel = pack_panel_view(&view);

    assert!(panel.active);
    assert_eq!(panel.pack_id.as_deref(), Some("roman_bridge_pack"));
    assert_eq!(panel.members.len(), 2);
    assert_eq!(panel.members[1].member_id, "tall_variant");
    assert_eq!(panel.members[1].name, "tall variant");
    assert!(panel.members[1].selected);

    assert_eq!(panel.shared_locks.len(), 1);
    assert_eq!(panel.shared_locks[0].target, "control:height");
    assert_eq!(panel.shared_locks[0].mode, "locked");
    assert_eq!(
        panel.shared_locks[0].reason.as_deref(),
        Some("Keep silhouettes aligned.")
    );

    assert_eq!(panel.shared_providers.len(), 1);
    assert_eq!(panel.shared_providers[0].role, "trim");
    assert_eq!(panel.shared_providers[0].provider_id, "trim.stone");

    let tall_overrides = panel
        .member_overrides
        .iter()
        .find(|row| row.member_id == "tall_variant")
        .expect("tall member overrides should be present");
    assert_eq!(tall_overrides.control_count, 1);
    assert_eq!(tall_overrides.provider_count, 1);
    assert_eq!(tall_overrides.total_count, 2);

    assert!(panel.validation.valid);
    assert_eq!(panel.validation.issue_count, 0);
    assert!(panel.export.enabled);
    assert_eq!(panel.export.profile.as_deref(), Some("game-ready"));
    assert!(panel.export.require_all_members);

    let action_labels = panel
        .actions
        .iter()
        .map(|action| action.action.label())
        .collect::<Vec<_>>();
    assert_eq!(
        action_labels,
        vec![
            "Add Current Asset",
            "Validate Batch",
            "Batch Export",
            "Contact Sheet"
        ]
    );
    assert!(
        !action_labels
            .iter()
            .any(|label| { label.contains("Marketplace") || label.contains("Publish") })
    );
}

#[test]
fn validation_warnings_gate_export_and_mark_contact_sheet_members() {
    let mut pack = fixture_pack();
    pack.members
        .get_mut("tall_variant")
        .expect("fixture member should exist")
        .family_content_ref = content_ref("family.other", 9);

    let validation = batch_validate_pack(&pack);
    assert!(!validation.valid);
    assert!(
        validation
            .issues
            .iter()
            .any(|issue| issue.code == "pack_member_family_mismatch")
    );

    let view = pack_view_from_document(pack, None);
    let panel = pack_panel_view(&view);
    assert!(!panel.validation.valid);
    assert!(!panel.export.enabled);
    assert_eq!(
        panel.export.disabled_reason.as_deref(),
        Some("Batch validation has blocking issues.")
    );
    assert!(
        panel
            .coherence_warnings
            .iter()
            .any(|warning| warning.subject == "members.tall_variant.family_content_ref")
    );

    let sheet = pack_contact_sheet(&view, 2);
    assert_eq!(sheet.columns, 2);
    assert_eq!(sheet.rows, 1);
    let tall_cell = sheet
        .cells
        .iter()
        .find(|cell| cell.member_id == "tall_variant")
        .expect("tall member cell should be present");
    assert_eq!(tall_cell.status, PackMemberStatus::NeedsAttention);
    let small_cell = sheet
        .cells
        .iter()
        .find(|cell| cell.member_id == "small")
        .expect("small member cell should be present");
    assert_eq!(small_cell.status, PackMemberStatus::Ready);
}

#[test]
fn batch_export_request_uses_existing_pack_compiler_job() {
    let view = pack_view_from_document(fixture_pack(), None);
    let request = batch_export_compile_request(&view, 77).expect("valid pack should export");

    match request {
        FoundryJobRequest::CompilePack { job_id, pack } => {
            assert_eq!(job_id, 77);
            assert_eq!(pack.pack_id, "roman_bridge_pack");
            assert_eq!(pack.export_profile.profile, "game-ready");
        }
        _ => panic!("expected pack compile request"),
    }

    let mut blocked_view = view;
    blocked_view.can_export = false;
    assert!(batch_export_compile_request(&blocked_view, 78).is_none());
}

fn fixture_pack() -> FoundryPackDocument {
    let family_ref = content_ref("family.bridge", 1);
    let style_ref = content_ref("style.roman", 2);
    let family_impl_ref = content_ref("family_impl.bridge", 3);
    let style_impl_ref = content_ref("style_impl.roman", 4);
    let profile_ref = content_ref("profile.bridge", 5);

    let mut small = foundry_document(
        "bridge_small",
        &family_ref,
        &style_ref,
        &family_impl_ref,
        &style_impl_ref,
        &profile_ref,
    );
    small
        .control_state
        .insert("height".to_owned(), ControlValue::Scalar(1.0));

    let mut tall = foundry_document(
        "bridge_tall",
        &family_ref,
        &style_ref,
        &family_impl_ref,
        &style_impl_ref,
        &profile_ref,
    );
    tall.control_state
        .insert("height".to_owned(), ControlValue::Scalar(1.0));
    tall.control_state
        .insert("span".to_owned(), ControlValue::Integer(3));
    tall.provider_overrides.insert(
        "arch".to_owned(),
        ProviderOverride {
            role: "arch".to_owned(),
            provider_ref: content_ref("arch.round", 6),
        },
    );

    let mut pack = FoundryPackDocument::new(
        "roman_bridge_pack",
        family_ref,
        style_ref,
        FoundryPackExportProfile {
            profile: "game-ready".to_owned(),
            require_all_members: true,
        },
    );
    pack.shared_controls
        .insert("height".to_owned(), ControlValue::Scalar(1.0));
    pack.shared_locks.push(FoundryLock {
        target: FoundryLockTarget::Control("height".to_owned()),
        mode: FoundryLockMode::Locked,
        reason: Some("Keep silhouettes aligned.".to_owned()),
    });
    pack.shared_provider_policy = SharedProviderPolicy::SharedExact(BTreeMap::from([(
        "trim".to_owned(),
        content_ref("trim.stone", 7),
    )]));
    pack.members.insert("small".to_owned(), small);
    pack.members.insert("tall_variant".to_owned(), tall);
    pack
}

fn foundry_document(
    document_id: &str,
    family_ref: &CatalogContentRef,
    style_ref: &CatalogContentRef,
    family_impl_ref: &CatalogContentRef,
    style_impl_ref: &CatalogContentRef,
    profile_ref: &CatalogContentRef,
) -> FoundryAssetDocument {
    FoundryAssetDocument::new(
        FoundryDocumentId(document_id.to_owned()),
        family_ref.clone(),
        style_ref.clone(),
        family_impl_ref.clone(),
        style_impl_ref.clone(),
        profile_ref.clone(),
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
