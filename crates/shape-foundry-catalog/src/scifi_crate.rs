//! Sci-Fi Industrial Crate compatibility shim.
//!
//! The public `sci-fi-crate` profile ID is preserved, but the executable
//! implementation now routes through the Cargo Case family.

use shape_foundry::{FoundryPartGroupDescriptor, built_in_part_group_descriptors_for_profile};

use crate::{FoundryFixtureCatalog, cargo_case};

pub const VENT_FOCUSED_LIMITED_REASON: &str =
    "Vents are controlled through Cargo Case Vent Density in this build.";
pub const PANEL_FOCUSED_LIMITED_REASON: &str =
    "Panels are controlled through Cargo Case Panel Complexity in this build.";
pub const HANDLE_FOCUSED_LIMITED_REASON: &str =
    "Handles are controlled through Cargo Case Handle Style in this build.";
pub const EDGE_TRIM_FOCUSED_LIMITED_REASON: &str =
    "Edge trim is controlled through Cargo Case Trim Style in this build.";
pub const FASTENER_FOCUSED_LIMITED_REASON: &str =
    "Fasteners are controlled through Cargo Case Detail Density in this build.";

/// Build the Sci-Fi Industrial Crate fixture catalog.
#[must_use]
pub fn fixture_catalog() -> FoundryFixtureCatalog {
    cargo_case::sci_fi_industrial_fixture_catalog()
}

/// Product-safe semantic part groups for the Sci-Fi Industrial Crate.
#[must_use]
pub fn part_group_descriptors() -> Vec<FoundryPartGroupDescriptor> {
    built_in_part_group_descriptors_for_profile(cargo_case::SCI_FI_CRATE_SLUG)
}
