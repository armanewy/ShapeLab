
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
