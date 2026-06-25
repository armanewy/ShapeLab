use std::collections::BTreeSet;

use shape_character::prepared::{
    HeroHumanReviewStatus, HeroQualityGateProfile, HeroQualityTier, HeroTemplateReviewManifest,
    HeroValidationStatus, PreparedHeroError, prepared_hero_template_v1,
};

#[test]
fn prepared_hero_template_v1_is_review_gated_prototype() {
    let template = prepared_hero_template_v1();

    template
        .validate()
        .expect("prepared hero template v1 validates");

    assert_eq!(template.template_id, "prepared-hero-template-v1");
    assert_eq!(
        template.quality_gate_profile.target_tier,
        HeroQualityTier::Prototype
    );
    assert_eq!(
        template.review_manifest.human_review_status,
        HeroHumanReviewStatus::Pending
    );
    assert!(!template.review_manifest.contact_sheet_evidence);
    assert!(template.external_mesh_claim.is_none());
    assert!(template.control_profile.controls.len() <= 7);

    let slot_ids = template
        .provider_slots
        .iter()
        .map(|slot| slot.slot_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        slot_ids,
        BTreeSet::from([
            "headgear",
            "shoulders",
            "torso_armor",
            "belt_skirt",
            "gauntlets",
            "boots",
            "weapon",
            "back_accessory",
            "hair_head_mass",
        ])
    );
}

#[test]
fn prepared_hero_showcase_claim_requires_human_contact_sheet_evidence() {
    let mut template = prepared_hero_template_v1();
    template.quality_gate_profile = HeroQualityGateProfile {
        target_tier: HeroQualityTier::Showcase,
        validation_status: HeroValidationStatus::PrototypeValidated,
        required_evidence: vec!["contact sheet".to_owned()],
    };
    template.review_manifest = HeroTemplateReviewManifest {
        achieved_tier: HeroQualityTier::Showcase,
        human_review_status: HeroHumanReviewStatus::Pending,
        contact_sheet_evidence: false,
        notes: vec!["not reviewed".to_owned()],
    };

    assert_eq!(
        template.validate(),
        Err(PreparedHeroError::UnsupportedQualityClaim)
    );
}
