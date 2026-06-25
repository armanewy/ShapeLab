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
            FoundryKitCardView {
                source_profile_slug: package.kit.source_profile_slug,
                display_name: package.kit.display_name,
                quality_badge: package.kit.quality_tier.label().to_owned(),
                style_name: package.style_pack.display_name,
                category_chips: package.kit.category_chips,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundry::ui::copy::first_forbidden_product_term;
    use shape_foundry::{
        materialize_foundation_draft_package, weapon_armor_foundation_draft_batch,
    };

    #[test]
    fn built_in_kit_cards_expose_product_safe_badges() {
        let cards = built_in_kit_card_views();
        assert_eq!(cards.len(), 11);
        assert!(cards.iter().all(|card| !card.display_name.is_empty()));
        assert!(cards.iter().all(|card| !card.style_name.is_empty()));
        assert!(cards.iter().all(|card| !card.category_chips.is_empty()));
        assert!(cards.iter().any(|card| card.quality_badge == "Usable"));
        assert!(cards.iter().any(|card| card.quality_badge == "Prototype"));
        assert!(cards.iter().all(|card| card.hidden_by_default));
        for card in cards {
            let labels = [
                card.source_profile_slug.as_deref().unwrap_or(""),
                card.display_name.as_str(),
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
            }
            for chip in &card.category_chips {
                assert_eq!(first_forbidden_product_term(chip), None);
            }
            if let Some(reason) = &card.hidden_reason {
                assert_eq!(first_forbidden_product_term(reason), None);
            }
        }
    }

    #[test]
    fn wave37_foundation_drafts_do_not_appear_in_novice_kit_cards() {
        let cards = built_in_kit_card_views();
        let card_names = cards
            .iter()
            .map(|card| card.display_name.as_str())
            .collect::<Vec<_>>();
        let card_slugs = cards
            .iter()
            .filter_map(|card| card.source_profile_slug.as_deref())
            .collect::<Vec<_>>();
        for draft in weapon_armor_foundation_draft_batch() {
            let materialized = materialize_foundation_draft_package(&draft)
                .expect("foundation batch draft should materialize internally");
            assert!(
                !card_names.contains(&draft.family_blueprint.display_name.as_str()),
                "{} must remain out of the novice kit catalog",
                draft.family_blueprint.display_name
            );
            assert!(
                !card_names.contains(&materialized.kit.display_name.as_str()),
                "{} materialized display name must remain out of the novice kit catalog",
                materialized.kit.display_name
            );
            assert!(
                !card_slugs.contains(&materialized.kit.kit_id.as_str()),
                "{} kit ID must not be used as a built-in novice card slug",
                materialized.kit.kit_id
            );
            assert!(
                materialized.kit.source_profile_slug.is_none(),
                "foundation drafts must not claim a built-in novice profile"
            );
        }
    }
}
