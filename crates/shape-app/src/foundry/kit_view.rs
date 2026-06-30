//! Product-safe kit card view data for the Visual Foundry Choose surface.

use shape_foundry::foundry_kit_visibility_decision;
use shape_foundry_catalog::built_in_foundry_kit_packages_with_labels;

/// Product-safe card data for one curated Foundry kit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoundryKitCardView {
    /// Built-in fixture slug when backed by bundled content.
    pub source_profile_slug: Option<String>,
    /// Kit display name.
    pub display_name: String,
    /// Stable asset-family ID.
    pub family_id: String,
    /// Asset-family display name.
    pub family_name: String,
    /// Quality tier badge.
    pub quality_badge: String,
    /// Style display name.
    pub style_name: String,
    /// Product-safe category chips.
    pub category_chips: Vec<String>,
    /// Review/visibility badge.
    pub verified_badge: String,
    /// Clay-preview badge.
    pub clay_preview_badge: String,
    /// Whether the default novice catalog hides the kit.
    pub hidden_by_default: bool,
    /// Product-safe hidden reason.
    pub hidden_reason: Option<String>,
}

/// Build product-safe kit card data for every built-in kit.
#[must_use]
pub(crate) fn built_in_kit_card_views() -> Vec<FoundryKitCardView> {
    built_in_foundry_kit_packages_with_labels()
        .into_iter()
        .map(|(_, package)| {
            let visibility =
                foundry_kit_visibility_decision(&package.kit, &package.review_manifest, false);
            let source_profile_slug = package.kit.source_profile_slug;
            let display_name =
                product_kit_display_name(source_profile_slug.as_deref(), &package.kit.display_name);
            let category_chips =
                product_category_chips(source_profile_slug.as_deref(), package.kit.category_chips);
            let family_id = package.family_blueprint.family_id.clone();
            FoundryKitCardView {
                source_profile_slug,
                display_name,
                family_id: family_id.clone(),
                family_name: product_family_display_name(
                    &family_id,
                    &package.family_blueprint.display_name,
                ),
                quality_badge: package.kit.quality_tier.label().to_owned(),
                style_name: package.style_pack.display_name,
                category_chips,
                verified_badge: if visibility.visible {
                    "Verified".to_owned()
                } else {
                    "Review pending".to_owned()
                },
                clay_preview_badge: if package.kit.preview_camera_policy.clay_preview_required {
                    "Clay preview".to_owned()
                } else {
                    "Preview pending".to_owned()
                },
                hidden_by_default: !visibility.visible,
                hidden_reason: visibility.reason,
            }
        })
        .collect()
}

fn product_kit_display_name(source_profile_slug: Option<&str>, display_name: &str) -> String {
    let _ = source_profile_slug;
    display_name.to_owned()
}

fn product_family_display_name(family_id: &str, display_name: &str) -> String {
    let _ = family_id;
    display_name.to_owned()
}

fn product_category_chips(
    source_profile_slug: Option<&str>,
    category_chips: Vec<String>,
) -> Vec<String> {
    let _ = source_profile_slug;
    category_chips
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundry::ui::copy::first_forbidden_product_term;

    #[test]
    fn built_in_kit_cards_expose_product_safe_badges() {
        let cards = built_in_kit_card_views();
        assert_eq!(cards.len(), 2);
        assert!(cards.iter().all(|card| !card.display_name.is_empty()));
        assert!(cards.iter().all(|card| !card.style_name.is_empty()));
        assert!(cards.iter().all(|card| !card.category_chips.is_empty()));
        assert_eq!(
            cards
                .iter()
                .map(|card| card.display_name.as_str())
                .collect::<Vec<_>>(),
            vec!["Box Primitive", "Lidded Box"]
        );
        assert!(cards.iter().all(|card| card.quality_badge == "Usable"));
        assert!(cards.iter().all(|card| card.hidden_by_default));
        for card in cards {
            let labels = [
                card.source_profile_slug.as_deref().unwrap_or(""),
                card.display_name.as_str(),
                card.family_id.as_str(),
                card.family_name.as_str(),
                card.quality_badge.as_str(),
                card.style_name.as_str(),
                card.verified_badge.as_str(),
                card.clay_preview_badge.as_str(),
            ];
            for label in labels {
                assert_eq!(
                    first_forbidden_product_term(label),
                    None,
                    "kit card label exposes internal product term: {label}"
                );
                assert!(
                    !label.to_ascii_lowercase().contains("mvp"),
                    "kit card label exposes planning copy: {label}"
                );
            }
            for chip in &card.category_chips {
                assert_eq!(first_forbidden_product_term(chip), None);
                assert!(
                    !chip.to_ascii_lowercase().contains("mvp"),
                    "kit category chip exposes planning copy: {chip}"
                );
            }
            if let Some(reason) = &card.hidden_reason {
                assert_eq!(first_forbidden_product_term(reason), None);
            }
        }
    }
}
