
/// Plain-language reason shown when a static surface package exists but visual
/// surface variation is not implemented.
pub const SURFACE_VISUAL_VARIATION_UNAVAILABLE_REASON: &str = "Surface package export exists for this kit. Visual surface variation needs textured previews and material candidate support.";

/// Plain-language reason shown when no surface package capability is known.
pub const SURFACE_PACKAGE_UNAVAILABLE_REASON: &str =
    "Surface variation needs a surface package for this kit.";

/// Product-facing export label for the current static-prop surface package.
pub const STATIC_PROP_SURFACE_PACKAGE_AVAILABLE_LABEL: &str =
    "Static prop surface package available";

/// Product-facing export description for the current static-prop surface package.
pub const STATIC_PROP_SURFACE_PACKAGE_DESCRIPTION: &str = "Exports a frozen mesh with UVs, material slots, simple procedural texture files, evidence images, and a validation report.";

/// Product-facing export caveat for full game-ready status.
pub const STATIC_PROP_FULL_READY_BLOCKED_NOTE: &str = "Still blocked from full game-ready status until manual review, engine import proof, and engine-native package handoff are complete.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartPreviewHint {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approximate_screen_anchor: Option<[u16; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlight_color_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartGroupCapability {
    pub shape_ready: bool,
    pub surface_ready: bool,
    pub detail_ready: bool,
    #[serde(default)]
    pub unavailable_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryPartGroupDescriptor {
    pub group_id: String,
    pub display_name: String,
    pub description: String,
    #[serde(default)]
    pub supported_channels: Vec<VariationChannel>,
    #[serde(default)]
    pub bound_control_ids: Vec<String>,
    #[serde(default)]
    pub bound_provider_roles: Vec<String>,
    pub lockable: bool,
    pub focusable: bool,
    pub preview_hint: PartPreviewHint,
    pub capability: PartGroupCapability,
}

impl FoundryPartGroupDescriptor {
    #[must_use]
    pub fn supports_channel(&self, channel: &VariationChannel) -> bool {
        self.supported_channels
            .iter()
            .any(|supported| supported == channel)
    }
}

/// Product-visible area a variation is allowed to affect.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum VariationScope {
    /// The whole authored asset.
    #[default]
    WholeAsset,
    /// A human-facing semantic part group.
    SemanticPartGroup {
        /// Stable group ID.
        group_id: String,
        /// Product-facing display name.
        display_name: String,
    },
    /// A future material slot.
    MaterialSlot {
        /// Stable slot ID.
        slot_id: String,
        /// Product-facing display name.
        display_name: String,
    },
    /// A human-facing detail zone.
    DetailZone {
        /// Stable zone ID.
        zone_id: String,
        /// Product-facing display name.
        display_name: String,
    },
    /// Reserved rig region.
    RigRegion {
        /// Stable region ID.
        region_id: String,
        /// Product-facing display name.
        display_name: String,
    },
    /// Reserved motion set.
    MotionSet {
        /// Stable motion-set ID.
        motion_set_id: String,
        /// Product-facing display name.
        display_name: String,
    },
    /// Custom/internal scope.
    Custom {
        /// Stable scope ID.
        scope_id: String,
        /// Product-facing display name.
        display_name: String,
    },
}

impl VariationScope {
    /// Return the product label for this scope.
    #[must_use]
    pub fn display_label(&self) -> &str {
        match self {
            Self::WholeAsset => "Whole Asset",
            Self::SemanticPartGroup { display_name, .. }
            | Self::MaterialSlot { display_name, .. }
            | Self::DetailZone { display_name, .. }
            | Self::RigRegion { display_name, .. }
            | Self::MotionSet { display_name, .. }
            | Self::Custom { display_name, .. } => display_name,
        }
    }

    /// Return true when this is a part-group focus scope.
    #[must_use]
    pub fn is_focus_part(&self) -> bool {
        matches!(self, Self::SemanticPartGroup { .. })
    }

    /// Return the stable part-group ID if this scope focuses a part group.
    #[must_use]
    pub fn semantic_part_group_id(&self) -> Option<&str> {
        match self {
            Self::SemanticPartGroup { group_id, .. } => Some(group_id),
            _ => None,
        }
    }
}

/// Product channel being varied.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "channel", rename_all = "snake_case")]
pub enum VariationChannel {
    /// Coherent whole-asset look. May combine supported shape and surface changes.
    #[default]
    CompleteLook,
    /// Geometry, proportions, silhouette, or structure.
    Shape,
    /// Material/surface slot changes. Reserved until surface payloads exist.
    Surface,
    /// Wear/weathering. Reserved until surface payloads exist.
    Wear,
    /// Detail-only changes.
    Detail,
    /// Reserved rig channel.
    Rig,
    /// Reserved motion channel.
    Motion,
    /// Reserved gameplay channel.
    Gameplay,
    /// Custom/internal channel.
    Custom {
        /// Stable channel ID.
        channel_id: String,
        /// Product-facing display name.
        display_name: String,
    },
}

impl VariationChannel {
    /// Return the product label for this channel.
    #[must_use]
    pub fn display_label(&self) -> &str {
        match self {
            Self::CompleteLook => "Complete Look",
            Self::Shape => "Shape",
            Self::Surface => "Surface",
            Self::Wear => "Wear",
            Self::Detail => "Detail",
            Self::Rig => "Rig",
            Self::Motion => "Motion",
            Self::Gameplay => "Gameplay",
            Self::Custom { display_name, .. } => display_name,
        }
    }

    /// Return true when this channel is visible to the default novice UI today.
    #[must_use]
    pub fn is_default_product_channel(&self, surface_supported: bool) -> bool {
        match self {
            Self::CompleteLook | Self::Shape => true,
            Self::Surface => surface_supported,
            Self::Wear
            | Self::Detail
            | Self::Rig
            | Self::Motion
            | Self::Gameplay
            | Self::Custom { .. } => false,
        }
    }
}

/// Product-facing surface package capability summary.
///
/// This is deliberately separate from Surface Lab's backend artifact structs so
/// Foundry can talk about product availability without claiming downstream
/// asset readiness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundrySurfaceCapabilityView {
    /// Source profile slug.
    pub profile_id: String,
    /// Whether a static-prop surface package is known for this kit.
    pub surface_package_available: bool,
    /// Whether a concrete surface payload is ready.
    pub surface_payload_ready: bool,
    /// Whether UV evidence is ready.
    pub uv_ready: bool,
    /// Number of material slots available without exposing raw slot IDs.
    pub material_slot_count: usize,
    /// Product-facing texture channel labels.
    pub texture_channels: Vec<String>,
    /// Whether headless textured preview/contact-sheet evidence exists.
    pub surface_visual_evidence_ready: bool,
    /// Whether the app can render and compare surface/material candidates.
    pub visual_surface_variation_ready: bool,
    /// Whether part-specific surface editing is truly available.
    pub focus_part_surface_ready: bool,
    /// Human-facing label.
    pub human_label: String,
    /// Plain-language unavailable reasons.
    pub unavailable_reasons: Vec<String>,
}

impl FoundrySurfaceCapabilityView {
    /// Build an unavailable capability summary for profiles without a known
    /// surface package.
    #[must_use]
    pub fn unavailable(profile_id: impl Into<String>, human_label: impl Into<String>) -> Self {
        Self {
            profile_id: profile_id.into(),
            surface_package_available: false,
            surface_payload_ready: false,
            uv_ready: false,
            material_slot_count: 0,
            texture_channels: Vec::new(),
            surface_visual_evidence_ready: false,
            visual_surface_variation_ready: false,
            focus_part_surface_ready: false,
            human_label: human_label.into(),
            unavailable_reasons: vec![SURFACE_PACKAGE_UNAVAILABLE_REASON.to_owned()],
        }
    }

    /// Return true only when Surface is a real visual candidate mode.
    #[must_use]
    pub fn surface_candidate_mode_available(&self) -> bool {
        self.visual_surface_variation_ready
    }

    /// Return the reason to show for the Surface variation mode when disabled.
    #[must_use]
    pub fn surface_mode_unavailable_reason(&self) -> &'static str {
        if self.surface_package_available {
            SURFACE_VISUAL_VARIATION_UNAVAILABLE_REASON
        } else {
            SURFACE_PACKAGE_UNAVAILABLE_REASON
        }
    }
}

/// Return the built-in surface capability known for a Foundry profile.
#[must_use]
pub fn built_in_surface_capability_for_profile(profile_id: &str) -> FoundrySurfaceCapabilityView {
    FoundrySurfaceCapabilityView::unavailable(profile_id.to_owned(), "Surface package")
}

#[must_use]
pub fn built_in_part_group_descriptors_for_profile(
    profile_id: &str,
) -> Vec<FoundryPartGroupDescriptor> {
    let normalized = profile_id.replace('_', "-").to_ascii_lowercase();
    if normalized.contains("box-primitive") {
        return vec![focus_group(
            "body",
            "Body",
            "Closed box-like volume.",
            &["proportions", "edge_softness"],
            &["body"],
            "Body",
            [50, 48],
        )];
    }
    Vec::new()
}

fn focus_group(
    group_id: &str,
    display_name: &str,
    description: &str,
    controls: &[&str],
    provider_roles: &[&str],
    preview_label: &str,
    anchor_percent: [u16; 2],
) -> FoundryPartGroupDescriptor {
    FoundryPartGroupDescriptor {
        group_id: group_id.to_owned(),
        display_name: display_name.to_owned(),
        description: description.to_owned(),
        supported_channels: vec![VariationChannel::Shape, VariationChannel::Detail],
        bound_control_ids: controls
            .iter()
            .map(|control| (*control).to_owned())
            .collect(),
        bound_provider_roles: provider_roles
            .iter()
            .map(|role| (*role).to_owned())
            .collect(),
        lockable: true,
        focusable: true,
        preview_hint: PartPreviewHint {
            label: preview_label.to_owned(),
            approximate_screen_anchor: Some(anchor_percent),
            highlight_color_name: Some("accent".to_owned()),
        },
        capability: PartGroupCapability {
            shape_ready: true,
            surface_ready: false,
            detail_ready: true,
            unavailable_reasons: vec![
                "Surface options need textured previews before they can be shown.".to_owned(),
            ],
        },
    }
}

/// Human-readable parse error for a Surface Lab capability sidecar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundrySurfaceCapabilityParseError {
    diagnostic: String,
}

impl FoundrySurfaceCapabilityParseError {
    fn new(diagnostic: impl Into<String>) -> Self {
        Self {
            diagnostic: diagnostic.into(),
        }
    }

    /// Human-readable diagnostic.
    #[must_use]
    pub fn diagnostic(&self) -> &str {
        &self.diagnostic
    }
}

impl fmt::Display for FoundrySurfaceCapabilityParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.diagnostic)
    }
}

impl std::error::Error for FoundrySurfaceCapabilityParseError {}

/// Parse a Surface Lab `surface/surface-capabilities.json` sidecar into the
/// product-facing Foundry capability view.
///
/// The sidecar can advertise Surface/Wear and material-slot capabilities, but
/// this parser does not turn those strings into enabled UI modes. Visual
/// Surface remains disabled until textured candidate previews and material
/// candidate support exist.
pub fn parse_foundry_surface_capability_sidecar_json(
    json: &str,
) -> Result<FoundrySurfaceCapabilityView, FoundrySurfaceCapabilityParseError> {
    let value: serde_json::Value = serde_json::from_str(json).map_err(|error| {
        FoundrySurfaceCapabilityParseError::new(format!(
            "Surface capability sidecar is not valid JSON: {error}"
        ))
    })?;
    if let Some(path) = absolute_local_path_in_json(&value) {
        return Err(FoundrySurfaceCapabilityParseError::new(format!(
            "Surface capability sidecar must not contain absolute local paths: {path}"
        )));
    }

    let sidecar: SurfaceCapabilitySidecar = serde_json::from_value(value).map_err(|error| {
        FoundrySurfaceCapabilityParseError::new(format!(
            "Surface capability sidecar has an unsupported shape: {error}"
        ))
    })?;
    validate_surface_capability_sidecar(&sidecar)?;
    let _supported_channels = sidecar.variation_channels_supported.to_channels()?;

    let texture_channels = sidecar
        .texture_channels
        .iter()
        .map(|channel| texture_channel_label(channel).map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    let visual_surface_variation_ready = false;
    let surface_visual_evidence_ready = sidecar.surface_visual_evidence_ready;
    let focus_part_surface_ready =
        sidecar.focus_part_surface_ready && visual_surface_variation_ready;
    let mut unavailable_reasons = Vec::new();
    if !visual_surface_variation_ready {
        push_unique_reason(
            &mut unavailable_reasons,
            SURFACE_VISUAL_VARIATION_UNAVAILABLE_REASON,
        );
    }
    if !focus_part_surface_ready {
        push_unique_reason(
            &mut unavailable_reasons,
            "Focus Part Surface needs part-specific surface editing support.",
        );
    }
    for reason in sidecar.unavailable_reasons {
        validate_product_text("unavailable_reasons", &reason)?;
        push_unique_reason(&mut unavailable_reasons, &reason);
    }

    Ok(FoundrySurfaceCapabilityView {
        profile_id: sidecar.profile_id,
        surface_package_available: true,
        surface_payload_ready: sidecar.surface_payload_ready,
        uv_ready: sidecar.uv_ready,
        material_slot_count: sidecar.material_slots.len(),
        texture_channels,
        surface_visual_evidence_ready,
        visual_surface_variation_ready,
        focus_part_surface_ready,
        human_label: sidecar.human_label,
        unavailable_reasons,
    })
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SurfaceCapabilitySidecar {
    schema_version: u32,
    profile_id: String,
    surface_payload_ready: bool,
    uv_ready: bool,
    material_slots: Vec<String>,
    texture_channels: Vec<String>,
    variation_channels_supported: SurfaceSidecarVariationChannels,
    #[serde(default)]
    surface_visual_evidence_ready: bool,
    focus_part_surface_ready: bool,
    human_label: String,
    unavailable_reasons: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum SurfaceSidecarVariationChannels {
    Flags(SurfaceSidecarVariationChannelFlags),
    Names(Vec<String>),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SurfaceSidecarVariationChannelFlags {
    surface: bool,
    wear: bool,
}

impl SurfaceSidecarVariationChannels {
    fn to_channels(&self) -> Result<Vec<VariationChannel>, FoundrySurfaceCapabilityParseError> {
        match self {
            Self::Flags(flags) => {
                let mut channels = Vec::new();
                if flags.surface {
                    channels.push(VariationChannel::Surface);
                }
                if flags.wear {
                    channels.push(VariationChannel::Wear);
                }
                Ok(channels)
            }
            Self::Names(names) => names
                .iter()
                .map(|name| match name.as_str() {
                    "surface" => Ok(VariationChannel::Surface),
                    "wear" => Ok(VariationChannel::Wear),
                    other => Err(FoundrySurfaceCapabilityParseError::new(format!(
                        "Unsupported surface variation channel '{other}'."
                    ))),
                })
                .collect(),
        }
    }
}

fn validate_surface_capability_sidecar(
    sidecar: &SurfaceCapabilitySidecar,
) -> Result<(), FoundrySurfaceCapabilityParseError> {
    if sidecar.schema_version != 1 {
        return Err(FoundrySurfaceCapabilityParseError::new(
            "Surface capability sidecar schema version is not supported.",
        ));
    }
    validate_identifier("profile_id", &sidecar.profile_id)?;
    validate_product_text("human_label", &sidecar.human_label)?;
    for (index, slot) in sidecar.material_slots.iter().enumerate() {
        validate_identifier(format!("material_slots.{index}"), slot)?;
    }
    if sidecar.surface_payload_ready
        && (!sidecar.uv_ready
            || sidecar.material_slots.is_empty()
            || sidecar.texture_channels.is_empty())
    {
        return Err(FoundrySurfaceCapabilityParseError::new(
            "Surface capability sidecar cannot mark payload ready without UV, material slot, and texture channel evidence.",
        ));
    }
    if sidecar.surface_visual_evidence_ready && !sidecar.surface_payload_ready {
        return Err(FoundrySurfaceCapabilityParseError::new(
            "Surface capability sidecar cannot mark visual evidence ready without a surface payload.",
        ));
    }
    Ok(())
}

fn validate_identifier(
    subject: impl Into<String>,
    value: &str,
) -> Result<(), FoundrySurfaceCapabilityParseError> {
    let subject = subject.into();
    if value.trim().is_empty() {
        return Err(FoundrySurfaceCapabilityParseError::new(format!(
            "{subject} must not be empty."
        )));
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.'))
    {
        return Err(FoundrySurfaceCapabilityParseError::new(format!(
            "{subject} must contain only ASCII letters, digits, dashes, underscores, or dots."
        )));
    }
    Ok(())
}

fn validate_product_text(
    subject: &str,
    value: &str,
) -> Result<(), FoundrySurfaceCapabilityParseError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(FoundrySurfaceCapabilityParseError::new(format!(
            "{subject} must not be empty."
        )));
    }
    if let Some(term) = forbidden_surface_product_term(trimmed) {
        return Err(FoundrySurfaceCapabilityParseError::new(format!(
            "{subject} contains implementation wording '{term}'."
        )));
    }
    Ok(())
}

fn forbidden_surface_product_term(text: &str) -> Option<&'static str> {
    let lowercase = text.to_ascii_lowercase();
    [
        "surfaceartifact",
        "uv set id",
        "material slot id",
        "provider id",
        "semantic id",
        "operation id",
        "scalar path",
        "gltf primitive",
        "compiler",
        "decompiler",
        "fragment",
        "remap",
        "conformance",
    ]
    .into_iter()
    .find(|term| lowercase.contains(term))
}

fn texture_channel_label(
    channel: &str,
) -> Result<&'static str, FoundrySurfaceCapabilityParseError> {
    match channel {
        "base_color" => Ok("Base color"),
        "metallic_roughness" => Ok("Metallic roughness"),
        "normal" => Ok("Normal"),
        "occlusion" => Ok("Occlusion"),
        "emissive" => Ok("Emissive"),
        other => Err(FoundrySurfaceCapabilityParseError::new(format!(
            "Unsupported texture channel '{other}'."
        ))),
    }
}

fn push_unique_reason(reasons: &mut Vec<String>, reason: &str) {
    if !reasons.iter().any(|existing| existing == reason) {
        reasons.push(reason.to_owned());
    }
}

fn absolute_local_path_in_json(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) if looks_like_absolute_local_path(text) => {
            Some(text.clone())
        }
        serde_json::Value::Array(values) => values.iter().find_map(absolute_local_path_in_json),
        serde_json::Value::Object(values) => values.values().find_map(absolute_local_path_in_json),
        _ => None,
    }
}

fn looks_like_absolute_local_path(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.starts_with("file://") || trimmed.starts_with("\\\\") || trimmed.starts_with('/') {
        return true;
    }
    let bytes = trimmed.as_bytes();
    bytes.len() >= 3
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
        && bytes[0].is_ascii_alphabetic()
}
