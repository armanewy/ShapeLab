//! Family-pack workspace panel boundary.
//!
//! This module intentionally stays UI-toolkit agnostic. The native pack
//! workspace is a secondary surface that summarizes the existing headless pack
//! contract and exposes host actions without taking over the single-asset flow.

use std::collections::{BTreeMap, BTreeSet};

use shape_foundry::{
    FoundryCommand, FoundryLock, FoundryLockMode, FoundryLockTarget, FoundryPackDocument,
    FoundryValidationReport, SharedProviderPolicy, validate_foundry_pack,
};

use super::super::{
    commands::FoundryAppCommand, jobs::FoundryJobRequest, view_model::FoundryPackView,
};

const DEFAULT_CONTACT_SHEET_COLUMNS: usize = 3;

/// UI-ready pack workspace snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackPanelView {
    /// Whether a pack workspace is available.
    pub active: bool,
    /// Current pack ID.
    pub pack_id: Option<String>,
    /// Named pack members.
    pub members: Vec<PackMemberRow>,
    /// Pack-level shared locks.
    pub shared_locks: Vec<PackSharedLockRow>,
    /// Pack-level shared provider choices.
    pub shared_providers: Vec<PackSharedProviderChoiceRow>,
    /// Member-specific override summaries.
    pub member_overrides: Vec<PackMemberOverrideRow>,
    /// Coherence warnings shown before export.
    pub coherence_warnings: Vec<PackCoherenceWarning>,
    /// Current batch validation status.
    pub validation: PackBatchValidation,
    /// Current batch export status.
    pub export: PackBatchExport,
    /// Compact contact sheet data.
    pub contact_sheet: PackContactSheet,
    /// Host actions available from the pack panel.
    pub actions: Vec<PackActionAvailability>,
}

/// One named member row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackMemberRow {
    /// Stable pack member ID.
    pub member_id: String,
    /// Human-facing member name.
    pub name: String,
    /// Source document ID.
    pub document_id: String,
    /// Whether this member is currently selected.
    pub selected: bool,
    /// Number of member-specific overrides.
    pub override_count: usize,
}

/// One shared lock row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackSharedLockRow {
    /// Stable target label.
    pub target: String,
    /// Lock mode label.
    pub mode: String,
    /// Optional human-facing reason.
    pub reason: Option<String>,
}

/// One shared provider choice row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackSharedProviderChoiceRow {
    /// Family role.
    pub role: String,
    /// Selected provider stable ID.
    pub provider_id: String,
}

/// Summary of member-specific changes that survive shared pack state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackMemberOverrideRow {
    /// Stable pack member ID.
    pub member_id: String,
    /// Controls with values that differ from pack shared controls.
    pub control_count: usize,
    /// Provider choices that differ from pack shared provider policy.
    pub provider_count: usize,
    /// Local recipe overrides on this member.
    pub local_recipe_count: usize,
    /// Combined override count.
    pub total_count: usize,
}

/// One warning surfaced before batch export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackCoherenceWarning {
    /// Stable warning subject.
    pub subject: String,
    /// Stable warning code.
    pub code: String,
    /// Human-readable warning.
    pub message: String,
}

/// Batch validation status for the pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackBatchValidation {
    /// True when the pack document currently validates.
    pub valid: bool,
    /// Number of validation issues.
    pub issue_count: usize,
    /// UI-ready issue rows.
    pub issues: Vec<PackCoherenceWarning>,
}

/// Batch export status for the pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackBatchExport {
    /// Export profile key, when a pack is open.
    pub profile: Option<String>,
    /// Whether every member is required to export successfully.
    pub require_all_members: bool,
    /// Whether batch export can be requested now.
    pub enabled: bool,
    /// Human-readable disabled reason.
    pub disabled_reason: Option<String>,
}

/// Compact contact sheet data for pack review.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackContactSheet {
    /// Contact sheet column count.
    pub columns: usize,
    /// Contact sheet row count.
    pub rows: usize,
    /// Ordered member cells.
    pub cells: Vec<PackContactSheetCell>,
}

/// One contact sheet member cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackContactSheetCell {
    /// Zero-based row.
    pub row: usize,
    /// Zero-based column.
    pub column: usize,
    /// Stable pack member ID.
    pub member_id: String,
    /// Human-facing member name.
    pub name: String,
    /// Source document ID.
    pub document_id: String,
    /// Validation/coherence status.
    pub status: PackMemberStatus,
    /// Number of member-specific overrides.
    pub override_count: usize,
    /// Whether this member is currently selected.
    pub selected: bool,
}

/// Contact sheet member status.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum PackMemberStatus {
    /// No member-scoped validation issues are known.
    Ready,
    /// Member-scoped validation issues need attention.
    NeedsAttention,
}

/// Pack panel actions. Marketplace publishing is intentionally absent.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum PackPanelAction {
    /// Add the current asset as a named pack member.
    AddCurrentAsset,
    /// Validate every pack member.
    ValidateBatch,
    /// Compile/export the batch through the existing pack pipeline.
    ExportBatch,
    /// Show a contact sheet for member review.
    ContactSheet,
}

impl PackPanelAction {
    /// Human-facing action label.
    #[must_use]
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::AddCurrentAsset => "Add Current Asset",
            Self::ValidateBatch => "Validate Batch",
            Self::ExportBatch => "Batch Export",
            Self::ContactSheet => "Contact Sheet",
        }
    }
}

/// Availability for one host action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackActionAvailability {
    /// Action kind.
    pub action: PackPanelAction,
    /// Whether the action is currently enabled.
    pub enabled: bool,
    /// Human-readable disabled reason.
    pub disabled_reason: Option<String>,
}

/// Build the generic command used by the host when the user adds the current
/// asset to a family pack.
#[must_use]
pub(crate) fn add_current_asset_to_pack_command(
    pack_id: impl Into<String>,
    member_id: impl Into<String>,
) -> FoundryAppCommand {
    FoundryAppCommand::run(FoundryCommand::AddCurrentToPack {
        pack_id: pack_id.into(),
        member_id: member_id.into(),
    })
}

/// Convert a semantic pack document into the app-level pack view.
#[must_use]
pub(crate) fn pack_view_from_document(
    pack: FoundryPackDocument,
    selected_member: Option<String>,
) -> FoundryPackView {
    let members = pack
        .members
        .iter()
        .map(|(member_id, document)| (member_id.clone(), document.document_id.clone()))
        .collect::<BTreeMap<_, _>>();
    let selected_member = selected_member
        .filter(|member_id| members.contains_key(member_id))
        .or_else(|| members.keys().next().cloned());
    let shared_provider_choices = shared_provider_choices(&pack);
    let shared_locks = pack.shared_locks.clone();
    let member_override_counts = member_override_counts(&pack);
    let validation = validate_foundry_pack(&pack);
    let coherent = validation.is_valid();
    let can_export = coherent && !members.is_empty();

    FoundryPackView {
        pack_id: Some(pack.pack_id.clone()),
        pack: Some(pack),
        members,
        selected_member,
        shared_locks,
        shared_provider_choices,
        member_override_counts,
        coherence_warnings: validation_warning_texts(&validation),
        coherent,
        can_export,
    }
}

/// Build the panel DTO from the current app-level pack view.
#[must_use]
pub(crate) fn pack_panel_view(pack_view: &FoundryPackView) -> PackPanelView {
    let validation_report = pack_view
        .pack
        .as_ref()
        .map(validate_foundry_pack)
        .unwrap_or_default();
    let validation = batch_validation_from_report(pack_view.pack.is_some(), &validation_report);
    let coherence_warnings = coherence_warnings(pack_view, &validation_report);
    let export = batch_export_status(pack_view, validation.valid, coherence_warnings.is_empty());
    let contact_sheet = pack_contact_sheet_with_report(
        pack_view,
        &validation_report,
        DEFAULT_CONTACT_SHEET_COLUMNS,
    );
    let active =
        pack_view.pack.is_some() || pack_view.pack_id.is_some() || !pack_view.members.is_empty();

    PackPanelView {
        active,
        pack_id: pack_view.pack_id.clone(),
        members: member_rows(pack_view),
        shared_locks: shared_lock_rows(pack_view),
        shared_providers: shared_provider_rows(pack_view),
        member_overrides: member_override_rows(pack_view),
        coherence_warnings,
        validation,
        export,
        contact_sheet,
        actions: action_availability(pack_view),
    }
}

/// Validate a pack for the batch workflow.
#[must_use]
pub(crate) fn batch_validate_pack(pack: &FoundryPackDocument) -> PackBatchValidation {
    let report = validate_foundry_pack(pack);
    batch_validation_from_report(true, &report)
}

/// Build the request that starts the existing headless pack compiler for batch
/// export. The export action itself remains disabled until the view says the
/// pack is coherent and exportable.
#[must_use]
pub(crate) fn batch_export_compile_request(
    pack_view: &FoundryPackView,
    job_id: u64,
) -> Option<FoundryJobRequest> {
    if !pack_panel_view(pack_view).export.enabled {
        return None;
    }
    Some(FoundryJobRequest::CompilePack {
        job_id,
        pack: Box::new(pack_view.pack.as_ref()?.clone()),
    })
}

/// Build a contact sheet with a caller-chosen maximum column count.
#[must_use]
pub(crate) fn pack_contact_sheet(
    pack_view: &FoundryPackView,
    max_columns: usize,
) -> PackContactSheet {
    let validation_report = pack_view
        .pack
        .as_ref()
        .map(validate_foundry_pack)
        .unwrap_or_default();
    pack_contact_sheet_with_report(pack_view, &validation_report, max_columns)
}

fn batch_validation_from_report(
    has_pack: bool,
    report: &FoundryValidationReport,
) -> PackBatchValidation {
    let issues = report
        .issues
        .iter()
        .map(|issue| PackCoherenceWarning {
            subject: issue.subject.clone(),
            code: issue.code.clone(),
            message: issue.message.clone(),
        })
        .collect::<Vec<_>>();
    PackBatchValidation {
        valid: has_pack && issues.is_empty(),
        issue_count: issues.len(),
        issues,
    }
}

fn batch_export_status(
    pack_view: &FoundryPackView,
    valid: bool,
    warnings_clear: bool,
) -> PackBatchExport {
    let Some(pack) = &pack_view.pack else {
        return PackBatchExport {
            profile: None,
            require_all_members: false,
            enabled: false,
            disabled_reason: Some("No pack workspace is open.".to_owned()),
        };
    };

    let disabled_reason = if !valid {
        Some("Batch validation has blocking issues.".to_owned())
    } else if !warnings_clear || !pack_view.coherent {
        Some("Pack coherence warnings must be resolved before export.".to_owned())
    } else if !pack_view.can_export {
        Some("Pack export is not available for the current snapshot.".to_owned())
    } else {
        None
    };

    PackBatchExport {
        profile: Some(pack.export_profile.profile.clone()),
        require_all_members: pack.export_profile.require_all_members,
        enabled: disabled_reason.is_none(),
        disabled_reason,
    }
}

fn action_availability(pack_view: &FoundryPackView) -> Vec<PackActionAvailability> {
    let panel_has_pack = pack_view.pack.is_some() || pack_view.pack_id.is_some();
    let export = {
        let validation_report = pack_view
            .pack
            .as_ref()
            .map(validate_foundry_pack)
            .unwrap_or_default();
        let validation = batch_validation_from_report(pack_view.pack.is_some(), &validation_report);
        let warnings_clear = coherence_warnings(pack_view, &validation_report).is_empty();
        batch_export_status(pack_view, validation.valid, warnings_clear)
    };
    let has_members = !pack_view.members.is_empty();

    vec![
        PackActionAvailability {
            action: PackPanelAction::AddCurrentAsset,
            enabled: panel_has_pack,
            disabled_reason: (!panel_has_pack).then(|| "Open or create a pack first.".to_owned()),
        },
        PackActionAvailability {
            action: PackPanelAction::ValidateBatch,
            enabled: has_members,
            disabled_reason: (!has_members).then(|| "Add at least one pack member.".to_owned()),
        },
        PackActionAvailability {
            action: PackPanelAction::ExportBatch,
            enabled: export.enabled,
            disabled_reason: export.disabled_reason,
        },
        PackActionAvailability {
            action: PackPanelAction::ContactSheet,
            enabled: has_members,
            disabled_reason: (!has_members).then(|| "Add at least one pack member.".to_owned()),
        },
    ]
}

fn member_rows(pack_view: &FoundryPackView) -> Vec<PackMemberRow> {
    if let Some(pack) = &pack_view.pack {
        let counts = member_override_counts(pack);
        return pack
            .members
            .iter()
            .map(|(member_id, document)| PackMemberRow {
                member_id: member_id.clone(),
                name: member_name(member_id),
                document_id: document.document_id.0.clone(),
                selected: pack_view.selected_member.as_deref() == Some(member_id),
                override_count: counts.get(member_id).copied().unwrap_or_default(),
            })
            .collect();
    }

    pack_view
        .members
        .iter()
        .map(|(member_id, document_id)| PackMemberRow {
            member_id: member_id.clone(),
            name: member_name(member_id),
            document_id: document_id.0.clone(),
            selected: pack_view.selected_member.as_deref() == Some(member_id),
            override_count: pack_view
                .member_override_counts
                .get(member_id)
                .copied()
                .unwrap_or_default(),
        })
        .collect()
}

fn shared_lock_rows(pack_view: &FoundryPackView) -> Vec<PackSharedLockRow> {
    let locks = pack_view
        .pack
        .as_ref()
        .map_or(pack_view.shared_locks.as_slice(), |pack| {
            pack.shared_locks.as_slice()
        });
    locks.iter().map(shared_lock_row).collect()
}

fn shared_lock_row(lock: &FoundryLock) -> PackSharedLockRow {
    PackSharedLockRow {
        target: lock_target_label(&lock.target),
        mode: lock_mode_label(lock.mode).to_owned(),
        reason: lock.reason.clone(),
    }
}

fn shared_provider_rows(pack_view: &FoundryPackView) -> Vec<PackSharedProviderChoiceRow> {
    let choices = pack_view
        .pack
        .as_ref()
        .map(shared_provider_choices)
        .unwrap_or_else(|| pack_view.shared_provider_choices.clone());
    choices
        .into_iter()
        .map(|(role, provider_id)| PackSharedProviderChoiceRow { role, provider_id })
        .collect()
}

fn member_override_rows(pack_view: &FoundryPackView) -> Vec<PackMemberOverrideRow> {
    if let Some(pack) = &pack_view.pack {
        return pack
            .members
            .iter()
            .map(|(member_id, document)| member_override_row(pack, member_id, document))
            .collect();
    }

    pack_view
        .member_override_counts
        .iter()
        .map(|(member_id, count)| PackMemberOverrideRow {
            member_id: member_id.clone(),
            control_count: 0,
            provider_count: 0,
            local_recipe_count: 0,
            total_count: *count,
        })
        .collect()
}

fn member_override_counts(pack: &FoundryPackDocument) -> BTreeMap<String, usize> {
    pack.members
        .iter()
        .map(|(member_id, document)| {
            (
                member_id.clone(),
                member_override_row(pack, member_id, document).total_count,
            )
        })
        .collect()
}

fn member_override_row(
    pack: &FoundryPackDocument,
    member_id: &str,
    document: &shape_foundry::FoundryAssetDocument,
) -> PackMemberOverrideRow {
    let control_count = document
        .control_state
        .iter()
        .filter(|(control_id, value)| pack.shared_controls.get(*control_id) != Some(*value))
        .count();
    let provider_count = match &pack.shared_provider_policy {
        SharedProviderPolicy::Independent => document.provider_overrides.len(),
        SharedProviderPolicy::SharedExact(providers) => document
            .provider_overrides
            .iter()
            .filter(|(role, provider)| providers.get(*role) != Some(&provider.provider_ref))
            .count(),
    };
    let local_recipe_count = document.local_recipe_overrides.len();
    let total_count = control_count + provider_count + local_recipe_count;

    PackMemberOverrideRow {
        member_id: member_id.to_owned(),
        control_count,
        provider_count,
        local_recipe_count,
        total_count,
    }
}

fn coherence_warnings(
    pack_view: &FoundryPackView,
    validation_report: &FoundryValidationReport,
) -> Vec<PackCoherenceWarning> {
    let mut seen_messages = BTreeSet::new();
    let mut warnings = Vec::new();
    for issue in &validation_report.issues {
        seen_messages.insert(issue.message.clone());
        warnings.push(PackCoherenceWarning {
            subject: issue.subject.clone(),
            code: issue.code.clone(),
            message: issue.message.clone(),
        });
    }
    for (index, message) in pack_view.coherence_warnings.iter().enumerate() {
        if seen_messages.insert(message.clone()) {
            warnings.push(PackCoherenceWarning {
                subject: format!("pack.coherence.{index}"),
                code: "pack_coherence_warning".to_owned(),
                message: message.clone(),
            });
        }
    }
    warnings
}

fn pack_contact_sheet_with_report(
    pack_view: &FoundryPackView,
    validation_report: &FoundryValidationReport,
    max_columns: usize,
) -> PackContactSheet {
    let issue_member_ids = validation_report
        .issues
        .iter()
        .filter_map(|issue| member_id_from_issue_subject(&issue.subject))
        .collect::<BTreeSet<_>>();
    let members = member_rows(pack_view);
    if members.is_empty() {
        return PackContactSheet {
            columns: 0,
            rows: 0,
            cells: Vec::new(),
        };
    }

    let columns = max_columns.max(1).min(members.len());
    let rows = members.len().div_ceil(columns);
    let cells = members
        .into_iter()
        .enumerate()
        .map(|(index, member)| PackContactSheetCell {
            row: index / columns,
            column: index % columns,
            status: if issue_member_ids.contains(member.member_id.as_str()) {
                PackMemberStatus::NeedsAttention
            } else {
                PackMemberStatus::Ready
            },
            member_id: member.member_id,
            name: member.name,
            document_id: member.document_id,
            override_count: member.override_count,
            selected: member.selected,
        })
        .collect();

    PackContactSheet {
        columns,
        rows,
        cells,
    }
}

fn member_id_from_issue_subject(subject: &str) -> Option<&str> {
    subject.strip_prefix("members.")?.split('.').next()
}

fn shared_provider_choices(pack: &FoundryPackDocument) -> BTreeMap<String, String> {
    match &pack.shared_provider_policy {
        SharedProviderPolicy::Independent => BTreeMap::new(),
        SharedProviderPolicy::SharedExact(providers) => providers
            .iter()
            .map(|(role, provider_ref)| (role.clone(), provider_ref.stable_id.clone()))
            .collect(),
    }
}

fn validation_warning_texts(validation: &FoundryValidationReport) -> Vec<String> {
    validation
        .issues
        .iter()
        .map(|issue| issue.message.clone())
        .collect()
}

fn member_name(member_id: &str) -> String {
    member_id.replace(['_', '-'], " ")
}

fn lock_target_label(target: &FoundryLockTarget) -> String {
    match target {
        FoundryLockTarget::Control(control_id) => format!("control:{control_id}"),
        FoundryLockTarget::Role(role) => format!("role:{role}"),
        FoundryLockTarget::Provider(role) => format!("provider:{role}"),
        FoundryLockTarget::Override(override_id) => format!("override:{override_id}"),
        FoundryLockTarget::ExportProfile(profile) => format!("export:{profile}"),
        FoundryLockTarget::Custom(key) => format!("custom:{key}"),
    }
}

fn lock_mode_label(mode: FoundryLockMode) -> &'static str {
    match mode {
        FoundryLockMode::Locked => "locked",
        FoundryLockMode::SearchProtected => "search protected",
    }
}
