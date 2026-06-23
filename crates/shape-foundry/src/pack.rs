//! Foundry pack contracts and compilation.
#![allow(clippy::result_large_err)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_family::LengthValue;
use shape_family_compile::{
    RecipeFragment,
    identity::{ContentFingerprint, FingerprintError, fingerprint_serializable},
};

use crate::{
    CATALOG_LOCK_KEY_CUSTOMIZER_PROFILE, CATALOG_LOCK_KEY_FAMILY, CATALOG_LOCK_KEY_FAMILY_IMPL,
    CATALOG_LOCK_KEY_STYLE, CATALOG_LOCK_KEY_STYLE_IMPL, CatalogContentRef, ControlValue,
    FOUNDRY_PACK_DOCUMENT_SCHEMA_VERSION, FoundryAssetDocument, FoundryCatalogLock,
    FoundryCatalogResolver, FoundryCompilationError, FoundryCompilationOptions,
    FoundryCompilationOutput, FoundryConformanceSummary, FoundryLock, FoundryLockTarget,
    ProviderOverride, SharedProviderPolicy::SharedExact, compile_foundry_document_for_pack_report,
    document_catalog_refs, validate_foundry_pack,
};

/// Policy for shared providers across a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SharedProviderPolicy {
    /// Members can choose providers independently.
    Independent,
    /// Members must use the pack's shared provider choices.
    SharedExact(BTreeMap<String, CatalogContentRef>),
}

/// Coherence policy for a foundry pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackCoherencePolicy {
    /// All members must share the exact family and style refs.
    ExactFamilyAndStyle,
    /// Members may share only family.
    SharedFamilyOnly,
    /// Pack-authored coherence key.
    Custom(String),
}

/// Export profile for a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryPackExportProfile {
    /// Export profile key.
    pub profile: String,
    /// Whether all members must export successfully.
    pub require_all_members: bool,
}

/// Pack-level semantic source document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryPackDocument {
    /// Foundry pack schema version.
    pub schema_version: u32,
    /// Stable pack ID.
    pub pack_id: String,
    /// Shared family reference.
    pub shared_family_ref: CatalogContentRef,
    /// Shared style reference.
    pub shared_style_ref: CatalogContentRef,
    /// Shared locks.
    pub shared_locks: Vec<FoundryLock>,
    /// Pack-authored shared control state injected into every member before compilation.
    pub shared_controls: BTreeMap<String, ControlValue>,
    /// Shared provider policy.
    pub shared_provider_policy: SharedProviderPolicy,
    /// Named member documents.
    pub members: BTreeMap<String, FoundryAssetDocument>,
    /// Coherence policy.
    pub coherence_policy: PackCoherencePolicy,
    /// Export profile.
    pub export_profile: FoundryPackExportProfile,
    /// Optional exact catalog lock.
    pub catalog_lock: Option<FoundryCatalogLock>,
}

impl FoundryPackDocument {
    /// Construct an empty exact-family/style pack.
    #[must_use]
    pub fn new(
        pack_id: impl Into<String>,
        shared_family_ref: CatalogContentRef,
        shared_style_ref: CatalogContentRef,
        export_profile: FoundryPackExportProfile,
    ) -> Self {
        Self {
            schema_version: FOUNDRY_PACK_DOCUMENT_SCHEMA_VERSION,
            pack_id: pack_id.into(),
            shared_family_ref,
            shared_style_ref,
            shared_locks: Vec::new(),
            shared_controls: BTreeMap::new(),
            shared_provider_policy: SharedProviderPolicy::Independent,
            members: BTreeMap::new(),
            coherence_policy: PackCoherencePolicy::ExactFamilyAndStyle,
            export_profile,
            catalog_lock: None,
        }
    }
}

/// Compile a foundry pack using default member compilation options.
pub fn compile_foundry_pack(
    pack: &FoundryPackDocument,
    resolver: &impl FoundryCatalogResolver,
) -> Result<FoundryPackCompilationOutput, FoundryPackCompilationError> {
    compile_foundry_pack_with_options(pack, resolver, FoundryCompilationOptions::default())
}

/// Compile every member in a foundry pack and return a deterministic pack report.
pub fn compile_foundry_pack_with_options(
    pack: &FoundryPackDocument,
    resolver: &impl FoundryCatalogResolver,
    options: FoundryCompilationOptions,
) -> Result<FoundryPackCompilationOutput, FoundryPackCompilationError> {
    let validation = validate_foundry_pack(pack);
    if !validation.is_valid() {
        return Err(FoundryPackCompilationError::PackValidationFailed(
            validation,
        ));
    }

    let mut member_outputs = BTreeMap::new();
    for (member_id, document) in &pack.members {
        let member_document = effective_member_document(pack, document);
        let output = compile_foundry_document_for_pack_report(&member_document, resolver, options)
            .map_err(
                |error| FoundryPackCompilationError::MemberCompilationFailed {
                    member_id: member_id.clone(),
                    error: Box::new(error),
                },
            )?;
        member_outputs.insert(member_id.clone(), output);
    }

    let report = build_pack_report(pack, &member_outputs)?;
    if !report.conformance_status.accepted {
        return Err(FoundryPackCompilationError::CoherenceFailed(Box::new(
            report,
        )));
    }

    let mut compiled_pack = pack.clone();
    compiled_pack.catalog_lock = shared_catalog_lock(pack, &member_outputs);

    Ok(FoundryPackCompilationOutput {
        pack: compiled_pack,
        member_outputs,
        report,
    })
}

/// Complete pack compilation output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryPackCompilationOutput {
    /// Pack document with shared catalog lock populated when all members agreed.
    pub pack: FoundryPackDocument,
    /// Per-member foundry compilation outputs keyed by pack member ID.
    pub member_outputs: BTreeMap<String, FoundryCompilationOutput>,
    /// Deterministic pack-level report.
    pub report: FoundryPackReport,
}

/// Foundry pack compilation failure.
#[derive(Debug)]
pub enum FoundryPackCompilationError {
    /// The semantic pack contract is invalid before member compilation.
    PackValidationFailed(crate::FoundryValidationReport),
    /// A member document failed to compile.
    MemberCompilationFailed {
        /// Member ID inside the pack.
        member_id: String,
        /// Member compilation error.
        error: Box<FoundryCompilationError>,
    },
    /// Members compiled, but pack coherence checks rejected the result.
    CoherenceFailed(Box<FoundryPackReport>),
    /// Deterministic pack fingerprinting failed.
    Fingerprint {
        /// Fingerprinted subject.
        subject: String,
        /// Error text.
        error: String,
    },
}

/// Deterministic report for one compiled pack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryPackReport {
    /// Stable pack ID.
    pub pack_id: String,
    /// Ordered member reports.
    pub members: Vec<FoundryPackMemberReport>,
    /// Controls that resolve to the same value for every member.
    pub shared_controls: Vec<FoundryPackSharedControlReport>,
    /// Member-specific control and provider differences.
    pub differences: Vec<FoundryPackDifferenceReport>,
    /// Aggregate triangle counts.
    pub triangle_totals: FoundryPackTriangleTotals,
    /// Coarse descriptor spread across compiled member geometry.
    pub visual_descriptor_spread: FoundryPackVisualDescriptorSpread,
    /// Pack-level conformance and coherence status.
    pub conformance_status: FoundryPackConformanceStatus,
    /// Deterministic fingerprint of the report contents.
    pub report_fingerprint: ContentFingerprint,
}

/// One compiled pack member row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryPackMemberReport {
    /// Member ID inside the pack.
    pub member_id: String,
    /// Source document ID.
    pub document_id: String,
    /// Family catalog reference used by this member.
    pub family_ref: CatalogContentRef,
    /// Style catalog reference used by this member.
    pub style_ref: CatalogContentRef,
    /// Selected providers keyed by family role.
    pub provider_choices: BTreeMap<String, String>,
    /// Member control state after shared controls are applied.
    pub controls: BTreeMap<String, ControlValue>,
    /// Triangle count in the compiled artifact.
    pub triangle_count: u64,
    /// Coarse deterministic visual descriptor.
    pub visual_descriptor: FoundryPackVisualDescriptor,
    /// Member conformance summary.
    pub conformance: FoundryConformanceSummary,
}

/// One shared control value across every pack member.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryPackSharedControlReport {
    /// Control ID.
    pub control_id: String,
    /// Shared value.
    pub value: ControlValue,
}

/// One member-specific difference from the pack baseline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryPackDifferenceReport {
    /// Member ID inside the pack.
    pub member_id: String,
    /// Stable difference subject.
    pub subject: String,
    /// Baseline value as deterministic text.
    pub baseline: Option<String>,
    /// Member value as deterministic text.
    pub value: Option<String>,
}

/// Aggregate triangle counts for a pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryPackTriangleTotals {
    /// Sum of all member triangles.
    pub total: u64,
    /// Minimum member triangle count.
    pub minimum_member: u64,
    /// Maximum member triangle count.
    pub maximum_member: u64,
    /// Per-member triangle counts.
    pub by_member: BTreeMap<String, u64>,
}

/// Coarse descriptor derived from compiled artifact metadata and mesh bounds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryPackVisualDescriptor {
    /// World-space bounding-box extent.
    pub bounds_extent: [f32; 3],
    /// World-space bounding-box volume.
    pub bounds_volume: f32,
    /// Compiled part count.
    pub part_count: u64,
    /// Polygon face count.
    pub polygon_face_count: u64,
    /// Triangle count.
    pub triangle_count: u64,
    /// Triangles per bounding-box volume.
    pub triangle_density: f32,
}

/// Descriptor spread across a pack.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryPackVisualDescriptorSpread {
    /// Per-member descriptors.
    pub by_member: BTreeMap<String, FoundryPackVisualDescriptor>,
    /// Largest normalized pairwise descriptor distance.
    pub maximum_pairwise_distance: f32,
}

/// Pack-level conformance status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryPackConformanceStatus {
    /// True only when every member compiled and all pack coherence checks passed.
    pub accepted: bool,
    /// Stable pack issue rows.
    pub issues: Vec<FoundryPackIssue>,
}

/// One pack-level issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryPackIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct PackReportFingerprintPayload<'a> {
    pack_id: &'a str,
    members: &'a [FoundryPackMemberReport],
    shared_controls: &'a [FoundryPackSharedControlReport],
    differences: &'a [FoundryPackDifferenceReport],
    triangle_totals: &'a FoundryPackTriangleTotals,
    visual_descriptor_spread: &'a FoundryPackVisualDescriptorSpread,
    conformance_status: &'a FoundryPackConformanceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct EdgeLanguageSignature {
    curve_family: String,
    allowed_profiles: Vec<String>,
    allow_asymmetry: bool,
    bevel_segments: u32,
    bevel_profile_micros: i64,
    bevel_width: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct ProviderVocabularySignature {
    by_role: BTreeMap<String, Vec<ProviderSemanticSignature>>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
enum ProviderVocabularySource {
    FamilyFragment,
    StylePrototype,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct ProviderSemanticSignature {
    source: ProviderVocabularySource,
    fingerprint: ContentFingerprint,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct ScaleFamilySignature {
    family_id: String,
    role_scale_units: BTreeMap<String, [String; 3]>,
}

fn effective_member_document(
    pack: &FoundryPackDocument,
    document: &FoundryAssetDocument,
) -> FoundryAssetDocument {
    let mut member = document.clone();
    let mut catalog_refs_changed = false;
    for (control_id, value) in &pack.shared_controls {
        member
            .control_state
            .insert(control_id.clone(), value.clone());
    }
    if let SharedExact(providers) = &pack.shared_provider_policy {
        for (role, provider_ref) in providers {
            member.provider_overrides.insert(
                role.clone(),
                ProviderOverride {
                    role: role.clone(),
                    provider_ref: provider_ref.clone(),
                },
            );
            catalog_refs_changed = true;
        }
    }
    for shared_lock in &pack.shared_locks {
        if !member
            .foundry_locks
            .iter()
            .any(|lock| lock.target == shared_lock.target)
        {
            member.foundry_locks.push(shared_lock.clone());
        }
    }
    if catalog_refs_changed || pack.catalog_lock.is_some() {
        let mut lock = member
            .catalog_lock
            .clone()
            .unwrap_or_else(|| FoundryCatalogLock::from_document_refs(&member));
        if catalog_refs_changed {
            lock.exact_refs = document_catalog_refs(&member);
        }
        if let Some(pack_lock) = &pack.catalog_lock {
            lock.exact_refs.extend(pack_lock.exact_refs.clone());
            for snapshot in &pack_lock.embedded_snapshots {
                if !lock
                    .embedded_snapshots
                    .iter()
                    .any(|existing| existing.content_ref == snapshot.content_ref)
                {
                    lock.embedded_snapshots.push(snapshot.clone());
                }
            }
            lock.compiler_version = pack_lock.compiler_version.clone();
            lock.catalog_version = pack_lock.catalog_version;
        }
        member.catalog_lock = Some(lock);
    }
    member
}

fn build_pack_report(
    pack: &FoundryPackDocument,
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) -> Result<FoundryPackReport, FoundryPackCompilationError> {
    let members = member_reports(outputs);
    let shared_controls = shared_controls(outputs);
    let differences = difference_reports(outputs);
    let triangle_totals = triangle_totals(&members);
    let visual_descriptor_spread = visual_descriptor_spread(&members);
    let conformance_status = conformance_status(pack, outputs, &members);
    let payload = PackReportFingerprintPayload {
        pack_id: &pack.pack_id,
        members: &members,
        shared_controls: &shared_controls,
        differences: &differences,
        triangle_totals: &triangle_totals,
        visual_descriptor_spread: &visual_descriptor_spread,
        conformance_status: &conformance_status,
    };
    let report_fingerprint = fingerprint_serializable(
        "shape-lab.foundry-pack-report.v1",
        "foundry_pack_report",
        &payload,
    )
    .map_err(pack_fingerprint_error)?;

    Ok(FoundryPackReport {
        pack_id: pack.pack_id.clone(),
        members,
        shared_controls,
        differences,
        triangle_totals,
        visual_descriptor_spread,
        conformance_status,
        report_fingerprint,
    })
}

fn member_reports(
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) -> Vec<FoundryPackMemberReport> {
    outputs
        .iter()
        .map(|(member_id, output)| FoundryPackMemberReport {
            member_id: member_id.clone(),
            document_id: output.document.document_id.0.clone(),
            family_ref: output.document.family_content_ref.clone(),
            style_ref: output.document.style_content_ref.clone(),
            provider_choices: provider_choices(output),
            controls: output.document.control_state.clone(),
            triangle_count: output.artifact.statistics.triangle_count,
            visual_descriptor: visual_descriptor(output),
            conformance: output.conformance_summary.clone(),
        })
        .collect()
}

fn provider_choices(output: &FoundryCompilationOutput) -> BTreeMap<String, String> {
    let mut choices = output
        .catalog
        .family_implementation
        .default_role_providers
        .clone();
    choices.extend(
        output
            .catalog
            .style_implementation
            .default_role_providers
            .clone(),
    );
    for report in &output.provider_override_reports {
        choices.insert(report.role.clone(), report.provider_id.clone());
    }
    choices
}

fn shared_controls(
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) -> Vec<FoundryPackSharedControlReport> {
    let Some((_, first)) = outputs.iter().next() else {
        return Vec::new();
    };
    first
        .document
        .control_state
        .iter()
        .filter(|(control_id, value)| {
            outputs
                .values()
                .all(|output| output.document.control_state.get(*control_id) == Some(*value))
        })
        .map(|(control_id, value)| FoundryPackSharedControlReport {
            control_id: control_id.clone(),
            value: value.clone(),
        })
        .collect()
}

fn difference_reports(
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) -> Vec<FoundryPackDifferenceReport> {
    let Some((baseline_member_id, baseline)) = outputs.iter().next() else {
        return Vec::new();
    };
    let baseline_providers = provider_choices(baseline);
    let control_ids = outputs
        .values()
        .flat_map(|output| output.document.control_state.keys().cloned())
        .collect::<BTreeSet<_>>();
    let provider_roles = outputs
        .values()
        .flat_map(provider_choices)
        .map(|(role, _)| role)
        .collect::<BTreeSet<_>>();

    let mut differences = Vec::new();
    for (member_id, output) in outputs {
        if member_id == baseline_member_id {
            continue;
        }
        for control_id in &control_ids {
            let baseline_value = baseline.document.control_state.get(control_id);
            let member_value = output.document.control_state.get(control_id);
            if baseline_value != member_value {
                differences.push(FoundryPackDifferenceReport {
                    member_id: member_id.clone(),
                    subject: format!("control_state.{control_id}"),
                    baseline: baseline_value.map(describe_control_value),
                    value: member_value.map(describe_control_value),
                });
            }
        }
        let member_providers = provider_choices(output);
        for role in &provider_roles {
            let baseline_value = baseline_providers.get(role);
            let member_value = member_providers.get(role);
            if baseline_value != member_value {
                differences.push(FoundryPackDifferenceReport {
                    member_id: member_id.clone(),
                    subject: format!("provider_overrides.{role}"),
                    baseline: baseline_value.cloned(),
                    value: member_value.cloned(),
                });
            }
        }
    }
    differences
}

fn triangle_totals(members: &[FoundryPackMemberReport]) -> FoundryPackTriangleTotals {
    let by_member = members
        .iter()
        .map(|member| (member.member_id.clone(), member.triangle_count))
        .collect::<BTreeMap<_, _>>();
    FoundryPackTriangleTotals {
        total: members.iter().map(|member| member.triangle_count).sum(),
        minimum_member: members
            .iter()
            .map(|member| member.triangle_count)
            .min()
            .unwrap_or(0),
        maximum_member: members
            .iter()
            .map(|member| member.triangle_count)
            .max()
            .unwrap_or(0),
        by_member,
    }
}

fn visual_descriptor(output: &FoundryCompilationOutput) -> FoundryPackVisualDescriptor {
    let bounds = output.artifact.combined_preview.mesh.bounds;
    let bounds_extent = if bounds.is_empty() {
        [0.0, 0.0, 0.0]
    } else {
        [
            (bounds.max[0] - bounds.min[0]).max(0.0),
            (bounds.max[1] - bounds.min[1]).max(0.0),
            (bounds.max[2] - bounds.min[2]).max(0.0),
        ]
    };
    let bounds_volume = bounds_extent[0] * bounds_extent[1] * bounds_extent[2];
    let triangle_density = if bounds_volume > 0.0 {
        output.artifact.statistics.triangle_count as f32 / bounds_volume
    } else {
        0.0
    };
    FoundryPackVisualDescriptor {
        bounds_extent,
        bounds_volume,
        part_count: output.artifact.statistics.part_count,
        polygon_face_count: output.artifact.statistics.polygon_face_count,
        triangle_count: output.artifact.statistics.triangle_count,
        triangle_density,
    }
}

fn visual_descriptor_spread(
    members: &[FoundryPackMemberReport],
) -> FoundryPackVisualDescriptorSpread {
    let by_member = members
        .iter()
        .map(|member| (member.member_id.clone(), member.visual_descriptor.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut maximum_pairwise_distance = 0.0_f32;
    for (left_index, left) in members.iter().enumerate() {
        for right in members.iter().skip(left_index + 1) {
            maximum_pairwise_distance = maximum_pairwise_distance.max(descriptor_distance(
                &left.visual_descriptor,
                &right.visual_descriptor,
            ));
        }
    }
    FoundryPackVisualDescriptorSpread {
        by_member,
        maximum_pairwise_distance,
    }
}

fn descriptor_distance(
    left: &FoundryPackVisualDescriptor,
    right: &FoundryPackVisualDescriptor,
) -> f32 {
    let mut total = 0.0_f32;
    for axis in 0..3 {
        total += normalized_delta(left.bounds_extent[axis], right.bounds_extent[axis]);
    }
    total += normalized_delta(left.bounds_volume, right.bounds_volume);
    total += normalized_delta(left.part_count as f32, right.part_count as f32);
    total += normalized_delta(
        left.polygon_face_count as f32,
        right.polygon_face_count as f32,
    );
    total += normalized_delta(left.triangle_count as f32, right.triangle_count as f32);
    total += normalized_delta(left.triangle_density, right.triangle_density);
    total / 8.0
}

fn normalized_delta(left: f32, right: f32) -> f32 {
    let denominator = left.abs().max(right.abs()).max(1.0);
    ((left - right).abs() / denominator).min(1.0)
}

fn conformance_status(
    pack: &FoundryPackDocument,
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
    members: &[FoundryPackMemberReport],
) -> FoundryPackConformanceStatus {
    let mut issues = Vec::new();
    check_member_document_ids(&mut issues, members);
    check_style_facets(&mut issues, outputs);
    check_provider_vocabulary(&mut issues, outputs);
    check_edge_language(&mut issues, outputs);
    check_detail_density_range(&mut issues, outputs);
    check_scale_family(&mut issues, outputs);
    check_duplicate_geometry(pack, &mut issues, outputs);
    for member in members {
        if !member.conformance.accepted {
            issues.push(FoundryPackIssue {
                subject: format!("members.{}.conformance", member.member_id),
                code: "pack_member_conformance_rejected".to_owned(),
                message: "Pack member final conformance was not accepted.".to_owned(),
            });
        }
    }
    FoundryPackConformanceStatus {
        accepted: issues.is_empty(),
        issues,
    }
}

fn check_member_document_ids(
    issues: &mut Vec<FoundryPackIssue>,
    members: &[FoundryPackMemberReport],
) {
    let mut owners = BTreeMap::<&str, &str>::new();
    for member in members {
        if let Some(first_member) =
            owners.insert(member.document_id.as_str(), member.member_id.as_str())
        {
            issues.push(FoundryPackIssue {
                subject: format!("members.{}.document_id", member.member_id),
                code: "duplicate_pack_member_document_id".to_owned(),
                message: format!(
                    "Member document ID `{}` is already used by member `{first_member}`.",
                    member.document_id
                ),
            });
        }
    }
}

fn check_style_facets(
    issues: &mut Vec<FoundryPackIssue>,
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) {
    let mut baseline: Option<(&str, ContentFingerprint)> = None;
    for (member_id, output) in outputs {
        let Some(facet) = output
            .catalog
            .style_kit
            .family_facets
            .get(&output.catalog.family.id)
        else {
            issues.push(FoundryPackIssue {
                subject: format!("members.{member_id}.style_facet"),
                code: "pack_missing_style_facet".to_owned(),
                message: "Member style kit does not expose a facet for its family.".to_owned(),
            });
            continue;
        };
        let Ok(fingerprint) =
            fingerprint_serializable("shape-lab.foundry-pack-style-facet.v1", member_id, facet)
        else {
            issues.push(FoundryPackIssue {
                subject: format!("members.{member_id}.style_facet"),
                code: "pack_style_facet_fingerprint_failed".to_owned(),
                message: "Member style facet could not be fingerprinted deterministically."
                    .to_owned(),
            });
            continue;
        };
        if let Some((baseline_member_id, baseline_fingerprint)) = baseline {
            if fingerprint != baseline_fingerprint {
                issues.push(FoundryPackIssue {
                    subject: format!("members.{member_id}.style_facet"),
                    code: "pack_style_facet_mismatch".to_owned(),
                    message: format!(
                        "Member style facet differs from baseline member `{baseline_member_id}`."
                    ),
                });
            }
        } else {
            baseline = Some((member_id.as_str(), fingerprint));
        }
    }
}

fn check_provider_vocabulary(
    issues: &mut Vec<FoundryPackIssue>,
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) {
    let mut baseline: Option<(&str, ProviderVocabularySignature)> = None;
    for (member_id, output) in outputs {
        let Ok(signature) = provider_vocabulary_signature(output) else {
            issues.push(FoundryPackIssue {
                subject: format!("members.{member_id}.provider_vocabulary"),
                code: "pack_provider_vocabulary_fingerprint_failed".to_owned(),
                message: "Member provider vocabulary could not be fingerprinted deterministically."
                    .to_owned(),
            });
            continue;
        };
        if let Some((baseline_member_id, baseline_signature)) = &baseline {
            if signature != *baseline_signature {
                issues.push(FoundryPackIssue {
                    subject: format!("members.{member_id}.provider_vocabulary"),
                    code: "pack_provider_vocabulary_mismatch".to_owned(),
                    message: format!(
                        "Member provider vocabulary differs from baseline member `{baseline_member_id}`."
                    ),
                });
            }
        } else {
            baseline = Some((member_id.as_str(), signature));
        }
    }
}

fn provider_vocabulary_signature(
    output: &FoundryCompilationOutput,
) -> Result<ProviderVocabularySignature, FingerprintError> {
    let mut by_role = BTreeMap::<String, Vec<ProviderSemanticSignature>>::new();
    for fragment in output.catalog.family_implementation.fragments.values() {
        by_role
            .entry(fragment.provided_role.clone())
            .or_default()
            .push(provider_semantic_signature(
                ProviderVocabularySource::FamilyFragment,
                fragment,
            )?);
    }
    for fragment in output.catalog.style_implementation.prototypes.values() {
        by_role
            .entry(fragment.provided_role.clone())
            .or_default()
            .push(provider_semantic_signature(
                ProviderVocabularySource::StylePrototype,
                fragment,
            )?);
    }
    for providers in by_role.values_mut() {
        providers.sort();
    }
    Ok(ProviderVocabularySignature { by_role })
}

fn provider_semantic_signature(
    source: ProviderVocabularySource,
    fragment: &RecipeFragment,
) -> Result<ProviderSemanticSignature, FingerprintError> {
    let mut semantic_fragment = fragment.clone();
    semantic_fragment.id.clear();
    let fingerprint = fingerprint_serializable(
        "shape-lab.foundry-pack-provider-vocabulary.v1",
        "provider_fragment",
        &semantic_fragment,
    )?;
    Ok(ProviderSemanticSignature {
        source,
        fingerprint,
    })
}

fn check_edge_language(
    issues: &mut Vec<FoundryPackIssue>,
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) {
    let mut baseline: Option<(&str, EdgeLanguageSignature)> = None;
    for (member_id, output) in outputs {
        let signature = edge_language_signature(output);
        if let Some((baseline_member_id, baseline_signature)) = &baseline {
            if signature != *baseline_signature {
                issues.push(FoundryPackIssue {
                    subject: format!("members.{member_id}.edge_language"),
                    code: "pack_edge_language_mismatch".to_owned(),
                    message: format!(
                        "Member edge language differs from baseline member `{baseline_member_id}`."
                    ),
                });
            }
        } else {
            baseline = Some((member_id.as_str(), signature));
        }
    }
}

fn edge_language_signature(output: &FoundryCompilationOutput) -> EdgeLanguageSignature {
    let kit = &output.catalog.style_kit;
    let facet_bevel = kit
        .family_facets
        .get(&output.catalog.family.id)
        .and_then(|facet| facet.policy_overrides.bevel_policy.as_ref());
    let bevel = facet_bevel.unwrap_or(&kit.bevel_policy);
    let mut allowed_profiles = kit.profile_language.allowed_profiles.clone();
    allowed_profiles.sort();
    EdgeLanguageSignature {
        curve_family: kit.profile_language.curve_family.clone(),
        allowed_profiles,
        allow_asymmetry: kit.profile_language.allow_asymmetry,
        bevel_segments: bevel.segments,
        bevel_profile_micros: quantize_unit(bevel.profile.normalized),
        bevel_width: describe_length_value(&bevel.width),
    }
}

fn check_detail_density_range(
    issues: &mut Vec<FoundryPackIssue>,
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) {
    const MAX_DETAIL_DENSITY_SPREAD: f32 = 0.35;

    let mut minimum = f32::INFINITY;
    let mut maximum = f32::NEG_INFINITY;
    let mut min_member = "";
    let mut max_member = "";
    for (member_id, output) in outputs {
        let density = detail_density(output);
        if density < minimum {
            minimum = density;
            min_member = member_id;
        }
        if density > maximum {
            maximum = density;
            max_member = member_id;
        }
    }
    if minimum.is_finite() && maximum.is_finite() && maximum - minimum > MAX_DETAIL_DENSITY_SPREAD {
        issues.push(FoundryPackIssue {
            subject: "members.detail_density".to_owned(),
            code: "pack_detail_density_range_mismatch".to_owned(),
            message: format!(
                "Detail density spread from member `{min_member}` to `{max_member}` exceeds the coherent pack range."
            ),
        });
    }
}

fn detail_density(output: &FoundryCompilationOutput) -> f32 {
    let kit = &output.catalog.style_kit;
    let repetition = kit
        .family_facets
        .get(&output.catalog.family.id)
        .and_then(|facet| facet.policy_overrides.repetition.as_ref())
        .unwrap_or(&kit.repetition);
    let detail_modules = kit
        .family_facets
        .get(&output.catalog.family.id)
        .map_or(0.0, |facet| facet.detail_modules.len() as f32 * 0.05);
    (repetition.density * 0.5 + kit.exaggeration.detail * 0.5 + detail_modules).clamp(0.0, 1.0)
}

fn check_scale_family(
    issues: &mut Vec<FoundryPackIssue>,
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) {
    let mut baseline: Option<(&str, ScaleFamilySignature)> = None;
    for (member_id, output) in outputs {
        let signature = scale_family_signature(output);
        if let Some((baseline_member_id, baseline_signature)) = &baseline {
            if signature != *baseline_signature {
                issues.push(FoundryPackIssue {
                    subject: format!("members.{member_id}.scale_family"),
                    code: "pack_scale_family_mismatch".to_owned(),
                    message: format!(
                        "Member scale family differs from baseline member `{baseline_member_id}`."
                    ),
                });
            }
        } else {
            baseline = Some((member_id.as_str(), signature));
        }
    }
}

fn scale_family_signature(output: &FoundryCompilationOutput) -> ScaleFamilySignature {
    let mut role_scale_units = BTreeMap::new();
    if let Some(facet) = output
        .catalog
        .style_kit
        .family_facets
        .get(&output.catalog.family.id)
    {
        for proportion in &facet.proportions {
            role_scale_units.insert(
                proportion.role.clone(),
                [
                    length_value_unit(&proportion.preferred_scale[0]).to_owned(),
                    length_value_unit(&proportion.preferred_scale[1]).to_owned(),
                    length_value_unit(&proportion.preferred_scale[2]).to_owned(),
                ],
            );
        }
    }
    ScaleFamilySignature {
        family_id: output.catalog.family.id.clone(),
        role_scale_units,
    }
}

fn check_duplicate_geometry(
    pack: &FoundryPackDocument,
    issues: &mut Vec<FoundryPackIssue>,
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) {
    if duplicate_geometry_is_intentional(pack) {
        return;
    }
    let mut owners = BTreeMap::<ContentFingerprint, &str>::new();
    for (member_id, output) in outputs {
        let Ok(fingerprint) = geometry_fingerprint(output) else {
            issues.push(FoundryPackIssue {
                subject: format!("members.{member_id}.geometry_fingerprint"),
                code: "pack_geometry_fingerprint_failed".to_owned(),
                message: "Member geometry could not be fingerprinted deterministically.".to_owned(),
            });
            continue;
        };
        if let Some(first_member) = owners.insert(fingerprint, member_id.as_str()) {
            issues.push(FoundryPackIssue {
                subject: format!("members.{member_id}.geometry_fingerprint"),
                code: "pack_duplicate_geometry".to_owned(),
                message: format!(
                    "Member geometry duplicates member `{first_member}` without an intentional duplicate policy."
                ),
            });
        }
    }
}

fn geometry_fingerprint(
    output: &FoundryCompilationOutput,
) -> Result<ContentFingerprint, FingerprintError> {
    fingerprint_serializable(
        "shape-lab.foundry-pack-geometry.v1",
        "combined_preview_mesh",
        &output.artifact.combined_preview.mesh,
    )
}

fn duplicate_geometry_is_intentional(pack: &FoundryPackDocument) -> bool {
    matches!(
        pack.coherence_policy,
        PackCoherencePolicy::Custom(ref key) if key == "allow_duplicate_geometry"
    ) || pack.shared_locks.iter().any(|lock| {
        matches!(
            &lock.target,
            FoundryLockTarget::Custom(key) if key == "allow_duplicate_geometry"
        )
    })
}

fn shared_catalog_lock(
    pack: &FoundryPackDocument,
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) -> Option<FoundryCatalogLock> {
    let first = outputs.values().next()?;
    let mut exact_refs = BTreeMap::new();
    exact_refs.insert(
        CATALOG_LOCK_KEY_FAMILY.to_owned(),
        shared_catalog_ref(outputs, CATALOG_LOCK_KEY_FAMILY)?.clone(),
    );
    match &pack.coherence_policy {
        PackCoherencePolicy::ExactFamilyAndStyle => {
            exact_refs.insert(
                CATALOG_LOCK_KEY_STYLE.to_owned(),
                shared_catalog_ref(outputs, CATALOG_LOCK_KEY_STYLE)?.clone(),
            );
        }
        PackCoherencePolicy::SharedFamilyOnly | PackCoherencePolicy::Custom(_) => {
            if let Some(style_ref) = shared_catalog_ref(outputs, CATALOG_LOCK_KEY_STYLE) {
                exact_refs.insert(CATALOG_LOCK_KEY_STYLE.to_owned(), style_ref.clone());
            }
        }
    }
    for key in [
        CATALOG_LOCK_KEY_FAMILY_IMPL,
        CATALOG_LOCK_KEY_STYLE_IMPL,
        CATALOG_LOCK_KEY_CUSTOMIZER_PROFILE,
    ] {
        if let Some(content_ref) = shared_catalog_ref(outputs, key) {
            exact_refs.insert(key.to_owned(), content_ref.clone());
        }
    }
    Some(FoundryCatalogLock {
        exact_refs,
        embedded_snapshots: Vec::new(),
        compiler_version: first.catalog.catalog_lock.compiler_version.clone(),
        catalog_version: first.catalog.catalog_lock.catalog_version,
    })
}

fn shared_catalog_ref<'a>(
    outputs: &'a BTreeMap<String, FoundryCompilationOutput>,
    key: &str,
) -> Option<&'a CatalogContentRef> {
    let first_ref = outputs
        .values()
        .next()?
        .catalog
        .catalog_lock
        .exact_refs
        .get(key)?;
    outputs
        .values()
        .all(|output| output.catalog.catalog_lock.exact_refs.get(key) == Some(first_ref))
        .then_some(first_ref)
}

fn describe_control_value(value: &ControlValue) -> String {
    match value {
        ControlValue::Scalar(value) => value.to_string(),
        ControlValue::Integer(value) => value.to_string(),
        ControlValue::Toggle(value) => value.to_string(),
        ControlValue::Choice(value) | ControlValue::Provider(value) => value.clone(),
    }
}

fn quantize_unit(value: f32) -> i64 {
    (value.clamp(0.0, 1.0) * 1_000_000.0).round() as i64
}

fn describe_length_value(value: &LengthValue) -> String {
    match value {
        LengthValue::Meters(value) => format!("meters:{}", quantize_length(*value)),
        LengthValue::FamilyUnits(value) => format!("family_units:{}", quantize_length(*value)),
        LengthValue::RelativeToRole { role, ratio } => {
            format!("relative_to_role:{role}:{}", quantize_length(*ratio))
        }
    }
}

fn length_value_unit(value: &LengthValue) -> &'static str {
    match value {
        LengthValue::Meters(_) => "meters",
        LengthValue::FamilyUnits(_) => "family_units",
        LengthValue::RelativeToRole { .. } => "relative_to_role",
    }
}

fn quantize_length(value: f32) -> i64 {
    (value * 1_000_000.0).round() as i64
}

fn pack_fingerprint_error(error: FingerprintError) -> FoundryPackCompilationError {
    match error {
        FingerprintError::Serialization { subject, error } => {
            FoundryPackCompilationError::Fingerprint { subject, error }
        }
        FingerprintError::NonFiniteNumber { subject } => FoundryPackCompilationError::Fingerprint {
            subject,
            error: "canonical pack report contained a non-finite number".to_owned(),
        },
    }
}
