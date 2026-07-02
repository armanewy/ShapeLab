use super::*;

pub(super) fn developer_preview_catalog_enabled() -> bool {
    env::var(PREVIEW_CATALOG_ENV_VAR).is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

pub(super) fn object_plan_review_enabled() -> bool {
    env::var(OBJECT_PLAN_REVIEW_ENV_VAR).is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

pub(super) fn family_studio_lite_enabled() -> bool {
    env::var(FAMILY_STUDIO_LITE_ENV_VAR).is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

pub(super) fn family_studio_lite_store_base_dir() -> PathBuf {
    env::temp_dir().join("shape-lab-family-studio-lite")
}
