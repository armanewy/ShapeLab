//! Shared property descriptor contracts for kernel-to-authoring bridges.

use serde::{Deserialize, Serialize};

/// Stable kernel kind for current direct primitive and composition profiles.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum KernelKind {
    /// Direct Box Primitive kernel.
    BoxPrimitive,
    /// Direct Flat Panel Primitive kernel.
    FlatPanelPrimitive,
    /// Direct Sphere Primitive kernel.
    SpherePrimitive,
    /// Composition-backed Panel with Knob profile.
    PanelWithKnobComposition,
}

/// Small finite Orchard control family exposed to product UI.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum OrchardControlFamily {
    /// Size-like stretch control.
    Stretch,
    /// Shape profile/corner/flattening control.
    Profile,
    /// Repeated band control.
    Band,
    /// Repetition/pattern control.
    Pattern,
    /// Attachment anchor or placement control.
    Attachment,
    /// Finite option control.
    Option,
}

/// Descriptor effect class that maps onto authoring operation families.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PropertyAuthoringEffect {
    /// Emits an authoring set-property operation.
    SetProperty,
    /// Emits an authoring reset-property operation.
    ResetProperty,
}

/// Product review importance for a property.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PropertyReviewImportance {
    /// Primary beginner-facing property.
    Primary,
    /// Secondary product-facing property.
    Secondary,
    /// Advanced property hidden from the primary path.
    Advanced,
}

/// High-level output affected by one property.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PropertyAffect {
    /// Primitive dimensions or proportions.
    Dimensions,
    /// Primitive profile/edge/flattening.
    Profile,
    /// Attachment placement or anchor relation.
    AttachmentPlacement,
    /// Composition membership or relationship semantics.
    Composition,
}

/// Descriptor value domain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "PascalCase", deny_unknown_fields)]
pub enum PropertyDescriptorDomain {
    /// Inclusive bounded length range.
    Length {
        /// Minimum value.
        minimum: f32,
        /// Maximum value.
        maximum: f32,
        /// UI step size.
        step: f32,
    },
    /// Inclusive bounded ratio range.
    Ratio {
        /// Minimum value.
        minimum: f32,
        /// Maximum value.
        maximum: f32,
        /// UI step size.
        step: f32,
    },
    /// Boolean domain.
    Boolean,
    /// Finite symbolic choices.
    Choice {
        /// Legal choice IDs.
        options: Vec<String>,
    },
    /// Inclusive bounded angle range in degrees.
    Angle {
        /// Minimum value in degrees.
        minimum_degrees: f32,
        /// Maximum value in degrees.
        maximum_degrees: f32,
        /// UI step size in degrees.
        step_degrees: f32,
    },
}

/// Descriptor default value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "PascalCase")]
pub enum PropertyDescriptorValue {
    /// Length value.
    Length(f32),
    /// Ratio value.
    Ratio(f32),
    /// Boolean value.
    Boolean(bool),
    /// Choice ID.
    Choice(String),
    /// Angle in degrees.
    Angle(f32),
}

/// Stable descriptor for one product-editable kernel property.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyDescriptor {
    /// Stable descriptor ID.
    pub id: String,
    /// Semantic authoring path, not a raw scalar path.
    pub path: String,
    /// Product-facing label.
    pub label: String,
    /// Beginner-facing description.
    pub beginner_description: String,
    /// Product-facing group name.
    pub group: String,
    /// Allowed domain.
    pub domain: PropertyDescriptorDomain,
    /// Authored default value.
    pub default_value: PropertyDescriptorValue,
    /// Whether edits can change topology.
    pub topology_changing: bool,
    /// High-level outputs affected by this property.
    pub affects: Vec<PropertyAffect>,
    /// Review importance.
    pub review_importance: PropertyReviewImportance,
    /// Orchard control family.
    pub control_family: OrchardControlFamily,
    /// Authoring effect class emitted by the property.
    pub authoring_effect: PropertyAuthoringEffect,
}

/// Descriptor for one product kernel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelDescriptor {
    /// Stable kernel kind.
    pub kind: KernelKind,
    /// Product-facing label.
    pub display_name: String,
    /// Product-safe summary.
    pub beginner_description: String,
    /// Property descriptors exposed by this kernel.
    pub properties: Vec<PropertyDescriptor>,
}
