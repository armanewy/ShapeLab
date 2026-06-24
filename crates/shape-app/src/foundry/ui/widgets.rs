//! Reusable widget contracts for the Visual Foundry product surface.

use egui::{Color32, RichText};

use super::{
    copy::first_forbidden_product_term,
    tokens::{VisualFoundryColors, VisualFoundryTokens},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ButtonTone {
    Primary,
    Secondary,
    Quiet,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActionSpec<'a> {
    pub label: &'a str,
    pub tone: ButtonTone,
    pub enabled: bool,
    pub disabled_reason: Option<&'a str>,
}

impl<'a> ActionSpec<'a> {
    #[must_use]
    pub(crate) const fn enabled(label: &'a str, tone: ButtonTone) -> Self {
        Self {
            label,
            tone,
            enabled: true,
            disabled_reason: None,
        }
    }

    #[must_use]
    pub(crate) const fn disabled(label: &'a str, tone: ButtonTone, reason: &'a str) -> Self {
        Self {
            label,
            tone,
            enabled: false,
            disabled_reason: Some(reason),
        }
    }

    pub(crate) fn validate(self) -> Result<(), WidgetSpecError> {
        validate_visible_label(self.label)?;
        if !self.enabled && self.disabled_reason.unwrap_or("").trim().is_empty() {
            return Err(WidgetSpecError::MissingDisabledReason {
                label: self.label.to_owned(),
            });
        }
        if let Some(reason) = self.disabled_reason {
            validate_visible_label(reason)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StatusTone {
    Neutral,
    Ready,
    Working,
    Warning,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StatusPillSpec<'a> {
    pub label: &'a str,
    pub tone: StatusTone,
}

impl<'a> StatusPillSpec<'a> {
    #[must_use]
    pub(crate) const fn new(label: &'a str, tone: StatusTone) -> Self {
        Self { label, tone }
    }

    pub(crate) fn validate(self) -> Result<(), WidgetSpecError> {
        validate_visible_label(self.label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BannerTone {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StatusBannerSpec<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub tone: BannerTone,
}

impl<'a> StatusBannerSpec<'a> {
    pub(crate) fn validate(self) -> Result<(), WidgetSpecError> {
        validate_visible_label(self.title)?;
        validate_visible_label(self.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SectionHeaderSpec<'a> {
    pub eyebrow: &'a str,
    pub title: &'a str,
    pub subtitle: Option<&'a str>,
}

impl<'a> SectionHeaderSpec<'a> {
    pub(crate) fn validate(self) -> Result<(), WidgetSpecError> {
        validate_visible_label(self.eyebrow)?;
        validate_visible_label(self.title)?;
        if let Some(subtitle) = self.subtitle {
            validate_visible_label(subtitle)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProfileCardSpec<'a> {
    pub title: &'a str,
    pub description: &'a str,
    pub action: ActionSpec<'a>,
}

impl<'a> ProfileCardSpec<'a> {
    pub(crate) fn validate(self) -> Result<(), WidgetSpecError> {
        validate_visible_label(self.title)?;
        validate_visible_label(self.description)?;
        self.action.validate()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PreviewCardSpec<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub selected: bool,
}

impl<'a> PreviewCardSpec<'a> {
    pub(crate) fn validate(self) -> Result<(), WidgetSpecError> {
        validate_visible_label(self.title)?;
        validate_visible_label(self.subtitle)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DirectionCardSpec<'a> {
    pub title: &'a str,
    pub subtitle: &'a str,
    pub badge: Option<&'a str>,
    pub action: ActionSpec<'a>,
}

impl<'a> DirectionCardSpec<'a> {
    pub(crate) fn validate(self) -> Result<(), WidgetSpecError> {
        validate_visible_label(self.title)?;
        validate_visible_label(self.subtitle)?;
        if let Some(badge) = self.badge {
            validate_visible_label(badge)?;
        }
        self.action.validate()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ControlCardSpec<'a> {
    pub label: &'a str,
    pub description: &'a str,
    pub value_label: &'a str,
    pub locked: bool,
    pub disabled_reason: Option<&'a str>,
}

impl<'a> ControlCardSpec<'a> {
    pub(crate) fn validate(self) -> Result<(), WidgetSpecError> {
        validate_visible_label(self.label)?;
        validate_visible_label(self.description)?;
        validate_visible_label(self.value_label)?;
        if let Some(reason) = self.disabled_reason {
            validate_visible_label(reason)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OptionTileSpec<'a> {
    pub label: &'a str,
    pub selected: bool,
    pub unavailable_reason: Option<&'a str>,
}

impl<'a> OptionTileSpec<'a> {
    pub(crate) fn validate(self) -> Result<(), WidgetSpecError> {
        validate_visible_label(self.label)?;
        if let Some(reason) = self.unavailable_reason {
            validate_visible_label(reason)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EmptyStateSpec<'a> {
    pub title: &'a str,
    pub message: &'a str,
    pub action: Option<ActionSpec<'a>>,
}

impl<'a> EmptyStateSpec<'a> {
    pub(crate) fn validate(self) -> Result<(), WidgetSpecError> {
        validate_visible_label(self.title)?;
        validate_visible_label(self.message)?;
        if let Some(action) = self.action {
            action.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InlineReasonSpec<'a> {
    pub message: &'a str,
}

impl<'a> InlineReasonSpec<'a> {
    pub(crate) fn validate(self) -> Result<(), WidgetSpecError> {
        validate_visible_label(self.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeyValueRowSpec<'a> {
    pub key: &'a str,
    pub value: &'a str,
}

impl<'a> KeyValueRowSpec<'a> {
    pub(crate) fn validate(self) -> Result<(), WidgetSpecError> {
        validate_visible_label(self.key)?;
        validate_visible_label(self.value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProgressPulseSpec<'a> {
    pub label: &'a str,
}

impl<'a> ProgressPulseSpec<'a> {
    pub(crate) fn validate(self) -> Result<(), WidgetSpecError> {
        validate_visible_label(self.label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActionFooterSpec<'a> {
    pub primary: ActionSpec<'a>,
    pub secondary: Option<ActionSpec<'a>>,
}

impl<'a> ActionFooterSpec<'a> {
    pub(crate) fn validate(self) -> Result<(), WidgetSpecError> {
        self.primary.validate()?;
        if let Some(secondary) = self.secondary {
            secondary.validate()?;
        }
        Ok(())
    }
}

pub(crate) struct ActionCardResponse {
    pub card: egui::Response,
    pub action: Option<egui::Response>,
}

pub(crate) struct ActionFooterResponse {
    pub footer: egui::Response,
    pub primary: egui::Response,
    pub secondary: Option<egui::Response>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WidgetSpecError {
    EmptyLabel,
    ForbiddenTerm { label: String, term: &'static str },
    MissingDisabledReason { label: String },
}

pub(crate) fn validate_visible_label(label: &str) -> Result<(), WidgetSpecError> {
    if label.trim().is_empty() {
        return Err(WidgetSpecError::EmptyLabel);
    }
    if let Some(term) = first_forbidden_product_term(label) {
        return Err(WidgetSpecError::ForbiddenTerm {
            label: label.to_owned(),
            term,
        });
    }
    Ok(())
}

pub(crate) fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    action_button(ui, &ActionSpec::enabled(label, ButtonTone::Primary))
}

pub(crate) fn secondary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    action_button(ui, &ActionSpec::enabled(label, ButtonTone::Secondary))
}

pub(crate) fn quiet_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    action_button(ui, &ActionSpec::enabled(label, ButtonTone::Quiet))
}

pub(crate) fn danger_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    action_button(ui, &ActionSpec::enabled(label, ButtonTone::Danger))
}

pub(crate) fn action_button(ui: &mut egui::Ui, spec: &ActionSpec<'_>) -> egui::Response {
    let colors = VisualFoundryTokens::dark().colors;
    let (fill, text, stroke) = button_colors(colors, spec.tone, spec.enabled);
    let button = egui::Button::new(RichText::new(spec.label).color(text))
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(6))
        .min_size(egui::vec2(88.0, 32.0));
    let response = ui.add_enabled(spec.enabled, button);
    if spec.enabled {
        response
    } else if let Some(reason) = spec.disabled_reason {
        response.on_disabled_hover_text(reason)
    } else {
        response
    }
}

pub(crate) fn status_pill(ui: &mut egui::Ui, spec: StatusPillSpec<'_>) -> egui::Response {
    let colors = VisualFoundryTokens::dark().colors;
    let (fill, text) = status_colors(colors, spec.tone);
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, fill.gamma_multiply(1.35)))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.label(RichText::new(spec.label).color(text).strong());
        })
        .response
}

pub(crate) fn status_banner(ui: &mut egui::Ui, spec: StatusBannerSpec<'_>) -> egui::Response {
    let colors = VisualFoundryTokens::dark().colors;
    let fill = match spec.tone {
        BannerTone::Info => colors.accent_soft,
        BannerTone::Success => colors.success.gamma_multiply(0.16),
        BannerTone::Warning => colors.warning.gamma_multiply(0.18),
        BannerTone::Error => colors.danger.gamma_multiply(0.18),
    };
    let accent = match spec.tone {
        BannerTone::Info => colors.accent_hover,
        BannerTone::Success => colors.success,
        BannerTone::Warning => colors.warning,
        BannerTone::Error => colors.danger,
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, accent.gamma_multiply(0.7)))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.strong(RichText::new(spec.title).color(colors.text));
            ui.label(RichText::new(spec.message).color(colors.text_muted));
        })
        .response
}

pub(crate) fn profile_card(ui: &mut egui::Ui, spec: ProfileCardSpec<'_>) -> ActionCardResponse {
    let mut action = None;
    let card = framed_card(ui, false, |ui| {
        ui.strong(spec.title);
        ui.label(RichText::new(spec.description).small());
        ui.add_space(6.0);
        action = Some(action_button(ui, &spec.action));
    });
    ActionCardResponse { card, action }
}

pub(crate) fn preview_card(ui: &mut egui::Ui, spec: PreviewCardSpec<'_>) -> egui::Response {
    framed_card(ui, spec.selected, |ui| {
        ui.strong(spec.title);
        ui.label(RichText::new(spec.subtitle).small());
    })
}

pub(crate) fn direction_card(ui: &mut egui::Ui, spec: DirectionCardSpec<'_>) -> ActionCardResponse {
    let mut action = None;
    let card = framed_card(ui, false, |ui| {
        if let Some(badge) = spec.badge {
            let _ = status_pill(ui, StatusPillSpec::new(badge, StatusTone::Working));
        }
        ui.strong(spec.title);
        ui.label(RichText::new(spec.subtitle).small());
        ui.add_space(6.0);
        action = Some(action_button(ui, &spec.action));
    });
    ActionCardResponse { card, action }
}

pub(crate) fn control_card(ui: &mut egui::Ui, spec: ControlCardSpec<'_>) -> egui::Response {
    framed_card(ui, spec.locked, |ui| {
        ui.horizontal(|ui| {
            ui.strong(spec.label);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(spec.value_label);
            });
        });
        ui.label(RichText::new(spec.description).small());
        if let Some(reason) = spec.disabled_reason {
            inline_reason(ui, InlineReasonSpec { message: reason });
        }
    })
}

pub(crate) fn option_tile(ui: &mut egui::Ui, spec: OptionTileSpec<'_>) -> egui::Response {
    let response = framed_card(ui, spec.selected, |ui| {
        ui.centered_and_justified(|ui| {
            ui.label(spec.label);
        });
    });
    if let Some(reason) = spec.unavailable_reason {
        response.on_hover_text(reason)
    } else {
        response
    }
}

pub(crate) fn empty_state(ui: &mut egui::Ui, spec: EmptyStateSpec<'_>) -> ActionCardResponse {
    let mut action_response = None;
    let card = framed_card(ui, false, |ui| {
        ui.strong(spec.title);
        ui.label(RichText::new(spec.message).small());
        if let Some(action) = spec.action {
            ui.add_space(8.0);
            action_response = Some(action_button(ui, &action));
        }
    });
    ActionCardResponse {
        card,
        action: action_response,
    }
}

pub(crate) fn progress_pulse(ui: &mut egui::Ui, spec: ProgressPulseSpec<'_>) -> egui::Response {
    ui.horizontal(|ui| {
        ui.spinner();
        ui.label(spec.label);
    })
    .response
}

pub(crate) fn action_footer(ui: &mut egui::Ui, spec: ActionFooterSpec<'_>) -> ActionFooterResponse {
    let mut primary = None;
    let mut secondary = None;
    let footer = ui
        .horizontal(|ui| {
            if let Some(secondary_spec) = spec.secondary {
                secondary = Some(action_button(ui, &secondary_spec));
            }
            primary = Some(action_button(ui, &spec.primary));
        })
        .response;
    ActionFooterResponse {
        footer,
        primary: primary.expect("primary footer action is always rendered"),
        secondary,
    }
}

pub(crate) fn section_header(ui: &mut egui::Ui, spec: SectionHeaderSpec<'_>) {
    let colors = VisualFoundryTokens::dark().colors;
    ui.label(
        RichText::new(spec.eyebrow.to_ascii_uppercase())
            .color(colors.accent_hover)
            .small(),
    );
    ui.strong(spec.title);
    if let Some(subtitle) = spec.subtitle {
        ui.label(RichText::new(subtitle).color(colors.text_muted).small());
    }
}

pub(crate) fn inline_reason(ui: &mut egui::Ui, spec: InlineReasonSpec<'_>) {
    ui.label(
        RichText::new(spec.message)
            .color(VisualFoundryTokens::dark().colors.text_muted)
            .small(),
    );
}

pub(crate) fn key_value_row(ui: &mut egui::Ui, spec: KeyValueRowSpec<'_>) {
    let colors = VisualFoundryTokens::dark().colors;
    ui.horizontal(|ui| {
        ui.label(RichText::new(spec.key).color(colors.text_muted));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(spec.value).color(colors.text));
        });
    });
}

fn status_colors(colors: VisualFoundryColors, tone: StatusTone) -> (Color32, Color32) {
    let fill = match tone {
        StatusTone::Neutral => colors.panel_elevated,
        StatusTone::Ready => colors.success.gamma_multiply(0.18),
        StatusTone::Working => colors.accent_soft,
        StatusTone::Warning => colors.warning.gamma_multiply(0.18),
        StatusTone::Blocked => colors.danger.gamma_multiply(0.18),
    };
    let text = match tone {
        StatusTone::Neutral => colors.text_muted,
        StatusTone::Ready => colors.success,
        StatusTone::Working => colors.accent_hover,
        StatusTone::Warning => colors.warning,
        StatusTone::Blocked => colors.danger,
    };
    (fill, text)
}

fn framed_card(
    ui: &mut egui::Ui,
    selected: bool,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    let colors = VisualFoundryTokens::dark().colors;
    let stroke = if selected {
        egui::Stroke::new(1.5, colors.accent)
    } else {
        egui::Stroke::new(1.0, colors.stroke)
    };
    let fill = if selected {
        colors.accent_soft
    } else {
        colors.panel_elevated
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, add_contents)
        .response
}

fn button_colors(
    colors: VisualFoundryColors,
    tone: ButtonTone,
    enabled: bool,
) -> (Color32, Color32, egui::Stroke) {
    if !enabled {
        return (
            colors.disabled_fill,
            colors.text_subtle,
            egui::Stroke::new(1.0, colors.stroke),
        );
    }

    match tone {
        ButtonTone::Primary => (
            colors.accent,
            Color32::WHITE,
            egui::Stroke::new(1.0, colors.accent_hover),
        ),
        ButtonTone::Secondary => (
            colors.panel_elevated,
            colors.text,
            egui::Stroke::new(1.0, colors.stroke_strong),
        ),
        ButtonTone::Quiet => (
            Color32::TRANSPARENT,
            colors.text_muted,
            egui::Stroke::new(1.0, Color32::TRANSPARENT),
        ),
        ButtonTone::Danger => (
            colors.danger,
            Color32::WHITE,
            egui::Stroke::new(1.0, colors.danger),
        ),
    }
}

#[cfg(test)]
mod tests {
    use crate::foundry::ui::tokens::{MIN_BODY_TEXT_CONTRAST, contrast_ratio};

    use super::*;

    #[test]
    fn disabled_actions_require_plain_language_reason() {
        let missing = ActionSpec {
            label: "Export Pack",
            tone: ButtonTone::Primary,
            enabled: false,
            disabled_reason: None,
        };
        assert_eq!(
            missing.validate(),
            Err(WidgetSpecError::MissingDisabledReason {
                label: "Export Pack".to_owned()
            })
        );

        let valid = ActionSpec::disabled(
            "Export Pack",
            ButtonTone::Primary,
            "Add at least one asset before exporting.",
        );
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn widget_specs_reject_implementation_terms() {
        let tile = OptionTileSpec {
            label: "provider ID",
            selected: false,
            unavailable_reason: None,
        };
        assert!(matches!(
            tile.validate(),
            Err(WidgetSpecError::ForbiddenTerm { .. })
        ));
    }

    #[test]
    fn primary_product_specs_validate() {
        let profile = ProfileCardSpec {
            title: "Roman Timber Bridge",
            description: "A sturdy bridge family with direction and width choices.",
            action: ActionSpec::enabled("Start", ButtonTone::Primary),
        };
        let direction = DirectionCardSpec {
            title: "Reinforced",
            subtitle: "Sturdy and substantial",
            badge: Some("Current"),
            action: ActionSpec::enabled("Choose Direction", ButtonTone::Primary),
        };
        let control = ControlCardSpec {
            label: "Deck Width",
            description: "Overall deck width",
            value_label: "0.45",
            locked: false,
            disabled_reason: None,
        };
        assert!(profile.validate().is_ok());
        assert!(direction.validate().is_ok());
        assert!(control.validate().is_ok());
    }

    #[test]
    fn action_footer_validates_all_actions() {
        let footer = ActionFooterSpec {
            primary: ActionSpec::enabled("Export Pack", ButtonTone::Primary),
            secondary: Some(ActionSpec::disabled(
                "Export",
                ButtonTone::Secondary,
                "Build the current model before exporting.",
            )),
        };
        assert!(footer.validate().is_ok());
    }

    #[test]
    fn filled_button_tones_meet_body_text_contrast() {
        let colors = VisualFoundryTokens::dark().colors;
        for tone in [
            ButtonTone::Primary,
            ButtonTone::Secondary,
            ButtonTone::Danger,
        ] {
            let (fill, text, _) = button_colors(colors, tone, true);
            assert!(
                contrast_ratio(text, fill) >= MIN_BODY_TEXT_CONTRAST,
                "{tone:?} button contrast was {:.2}",
                contrast_ratio(text, fill)
            );
        }
        let (fill, text, _) = button_colors(colors, ButtonTone::Primary, false);
        assert!(
            contrast_ratio(text, fill) >= MIN_BODY_TEXT_CONTRAST,
            "disabled button contrast was {:.2}",
            contrast_ratio(text, fill)
        );
    }
}
