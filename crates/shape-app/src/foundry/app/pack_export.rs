use super::*;

pub(super) fn pack_member_display_name(member: &pack::PackMemberRow) -> String {
    let title = asset_title_from_id(&member.document_id);
    if title == "Shape Lab Project" {
        "Pack asset".to_owned()
    } else {
        title.to_owned()
    }
}

pub(super) fn pack_cell_display_name(cell: &pack::PackContactSheetCell) -> String {
    let title = asset_title_from_id(&cell.document_id);
    if title == "Shape Lab Project" {
        "Pack asset".to_owned()
    } else {
        title.to_owned()
    }
}

pub(super) fn show_pack_contact_sheet(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_preview: Option<&FoundryPreviewImage>,
    current_build: Option<&FoundryBuildStamp>,
    sheet: &pack::PackContactSheet,
) {
    if sheet.cells.is_empty() {
        product_empty_state(
            ui,
            "Pack is empty",
            "Add the current asset to start a pack.",
        );
        return;
    }

    let columns = sheet.columns.max(1);
    for row in 0..sheet.rows {
        ui.columns(columns, |column_uis| {
            for cell in sheet.cells.iter().filter(|cell| cell.row == row) {
                let column_index = cell.column.min(column_uis.len().saturating_sub(1));
                product_card(&mut column_uis[column_index], cell.selected, |ui| {
                    ui.set_min_height(144.0);
                    show_pack_cell_thumbnail(
                        ui,
                        texture_cache,
                        current_preview,
                        current_build,
                        cell,
                    );
                    ui.add_space(6.0);
                    ui.label(RichText::new(pack_cell_display_name(cell)).strong());
                    let status = match cell.status {
                        pack::PackMemberStatus::Ready => "Ready",
                        pack::PackMemberStatus::NeedsAttention => "Needs attention",
                    };
                    ui.label(RichText::new(status).small());
                    if cell.selected {
                        ui.weak("Current");
                    }
                    if cell.override_count > 0 {
                        ui.weak(format!("{} adjustment(s)", cell.override_count));
                    }
                });
            }
        });
        ui.add_space(8.0);
    }
}

pub(super) fn show_pack_cell_thumbnail(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_preview: Option<&FoundryPreviewImage>,
    current_build: Option<&FoundryBuildStamp>,
    cell: &pack::PackContactSheetCell,
) {
    if cell.selected
        && let Some(preview) = current_preview
    {
        let preview_id = format!("pack-current-{}", preview.preview_id);
        show_rgba_preview(
            ui,
            texture_cache,
            FoundryPreviewDraw {
                preview_id: &preview_id,
                build: preview.build.as_ref().or(current_build),
                rgba8: &preview.rgba8,
                width: preview.width,
                height: preview.height,
                max_edge: 88.0,
            },
        );
        return;
    }

    pack_thumbnail_placeholder(ui, cell);
}

pub(super) fn pack_thumbnail_placeholder(ui: &mut egui::Ui, cell: &pack::PackContactSheetCell) {
    let colors = VisualFoundryTokens::dark().colors;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(88.0, 62.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(6), colors.panel_subtle);
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(6),
        egui::Stroke::new(1.0, colors.stroke_strong),
        egui::StrokeKind::Inside,
    );
    let band_height = rect.height() * 0.34;
    let band = egui::Rect::from_min_max(
        egui::pos2(rect.left(), rect.bottom() - band_height),
        rect.right_bottom(),
    );
    ui.painter()
        .rect_filled(band, egui::CornerRadius::same(4), colors.accent_soft);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        pack_thumbnail_marker(cell),
        egui::FontId::proportional(11.0),
        colors.text_muted,
    );
}

pub(super) fn pack_thumbnail_marker(cell: &pack::PackContactSheetCell) -> &'static str {
    if cell.selected { "Current" } else { "Preview" }
}

pub(super) fn show_export_readiness_panel(
    ui: &mut egui::Ui,
    can_export_current: bool,
    pack_view: &crate::foundry::view_model::FoundryPackView,
    pack_panel: &pack::PackPanelView,
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    product_card(ui, can_export_current || pack_panel.export.enabled, |ui| {
        let colors = VisualFoundryTokens::dark().colors;
        ui.label(RichText::new(export_readiness_label(can_export_current)).color(colors.text));
        ui.label(
            RichText::new(export_readiness_detail(
                can_export_current,
                pack_panel.export.enabled,
            ))
            .color(colors.text_muted)
            .small(),
        );
        ui.label(
            RichText::new(BOX_PRIMITIVE_EXPORT_LIMITATION)
                .color(colors.text_muted)
                .small(),
        );
        ui.add_space(10.0);
        export_checklist_row(ui, "Model prepared", can_export_current, NEED_MODEL_REASON);
        export_checklist_row(
            ui,
            "Asset ready",
            can_export_current,
            "Prepare the current model first.",
        );
        export_checklist_row(
            ui,
            "Pack members",
            !pack_panel.members.is_empty(),
            NEED_PACK_MEMBER_REASON,
        );
        ui.add_space(12.0);
        ui.label(RichText::new("Export options").color(colors.text).strong());
        ui.add_space(6.0);
        if action_button(
            ui,
            &action_spec(
                can_export_current,
                ACTION_EXPORT_CURRENT_ASSET,
                ButtonTone::Primary,
                NEED_MODEL_REASON,
            ),
        )
        .clicked()
            && let Some(out_dir) = select_asset_export_dir()
        {
            commands.push(FoundryAppCommand::run(FoundryCommand::Export {
                profile: "default".to_owned(),
                out_dir: Some(out_dir.to_string_lossy().to_string()),
            }));
        }
        if !can_export_current {
            ui.label(
                RichText::new(NEED_MODEL_REASON)
                    .color(colors.warning)
                    .small(),
            );
        }
        ui.add_space(6.0);
        if action_button(
            ui,
            &action_spec(
                pack_panel.export.enabled,
                ACTION_EXPORT_PACK,
                ButtonTone::Secondary,
                NEED_PACK_MEMBER_REASON,
            ),
        )
        .clicked()
            && let Some(out_dir) = select_pack_export_dir()
            && let Some(command) = pack::batch_export_command(pack_view, out_dir)
        {
            commands.push(command);
        }
        if !pack_panel.export.enabled {
            let reason = if pack_panel.members.is_empty() {
                NEED_PACK_MEMBER_REASON.to_owned()
            } else if let Some(reason) = &pack_panel.export.disabled_reason {
                product_panel_message(reason, "Resolve pack warnings before export.")
            } else {
                "Resolve pack warnings before export.".to_owned()
            };
            ui.label(RichText::new(reason).color(colors.warning).small());
        }
    });
    commands
}

pub(super) fn export_checklist_row(ui: &mut egui::Ui, label: &str, ready: bool, reason: &str) {
    let colors = VisualFoundryTokens::dark().colors;
    ui.horizontal_wrapped(|ui| {
        let status = if ready { "Ready" } else { "Blocked" };
        ui.label(RichText::new(status).color(if ready {
            colors.success
        } else {
            colors.warning
        }));
        ui.label(RichText::new(label).color(colors.text));
        if !ready {
            ui.label(RichText::new(reason).color(colors.text_muted).small());
        }
    });
}

pub(super) fn pack_readiness_label(view: &pack::PackPanelView) -> &'static str {
    if view.export.enabled {
        "Export ready"
    } else if view.members.is_empty() {
        "Add assets to export"
    } else {
        "Needs attention"
    }
}

pub(super) fn pack_readiness_detail(view: &pack::PackPanelView) -> String {
    if view.export.enabled {
        "All assets are ready for pack export.".to_owned()
    } else if let Some(reason) = &view.export.disabled_reason {
        product_panel_message(reason, "Resolve pack warnings before export.")
    } else {
        "Add at least one asset before exporting a pack.".to_owned()
    }
}

pub(super) fn export_readiness_label(can_export: bool) -> &'static str {
    if can_export {
        "Current asset ready"
    } else {
        "Needs a model first"
    }
}

pub(super) fn export_readiness_detail(can_export: bool, pack_ready: bool) -> &'static str {
    if can_export && pack_ready {
        "Export this asset here, or export the prepared pack from the Pack drawer."
    } else if can_export {
        "Export the current asset as an individual result."
    } else {
        "Prepare the current asset before exporting."
    }
}

pub(super) fn unique_pack_member_id(
    pack: &crate::foundry::view_model::FoundryPackView,
    base: &str,
) -> String {
    let base = if base.trim().is_empty() {
        "member"
    } else {
        base.trim()
    };
    if !pack.members.contains_key(base) {
        return base.to_owned();
    }

    for index in 2.. {
        let candidate = format!("{base}-{index}");
        if !pack.members.contains_key(&candidate) {
            return candidate;
        }
    }

    unreachable!("unbounded member suffix search should always find an unused id")
}

impl FoundryDesktopApp {
    pub(super) fn show_export(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let (export_title, export_detail) = self.current_export_copy();
        section_header(
            ui,
            SectionHeaderSpec {
                eyebrow: "Export",
                title: export_title,
                subtitle: Some(export_detail),
            },
        );
        ui.add_space(10.0);
        if self.state.document.is_none() {
            commands.extend(self.show_choose_asset_empty_state(
                ui,
                "Choose an asset first",
                "Choose Box Primitive, Lidded Box, Flat Panel Primitive, Sphere Primitive, Hinged Panel, Panel with Knob, or open a project before exporting.",
            ));
            return commands;
        }
        if let Some((export_copy, full_ready_copy)) = self.material_look_export_copy() {
            product_card(ui, false, |ui| {
                let colors = VisualFoundryTokens::dark().colors;
                ui.label(
                    RichText::new(MATERIAL_LOOK_SECTION_TITLE)
                        .color(colors.accent_hover)
                        .small()
                        .strong(),
                );
                ui.add(egui::Label::new(RichText::new(export_copy).color(colors.warning)).wrap());
                ui.add(
                    egui::Label::new(RichText::new(full_ready_copy).color(colors.text_muted))
                        .wrap(),
                );
            });
            ui.add_space(10.0);
        }
        let can_export = self.state.current_output.is_some();
        let pack_view = self.state.pack.clone();
        let pack_panel = pack::pack_panel_view(&pack_view);
        let wide_layout = ui.available_width() >= 980.0;
        if wide_layout {
            ui.columns(2, |columns| {
                product_card(&mut columns[0], false, |ui| {
                    ui.label(
                        RichText::new("CURRENT ASSET")
                            .color(VisualFoundryTokens::dark().colors.accent_hover)
                            .small(),
                    );
                    let _ = self.show_current_preview_sized(ui, 420.0);
                });
                commands.extend(show_export_readiness_panel(
                    &mut columns[1],
                    can_export,
                    &pack_view,
                    &pack_panel,
                ));
            });
        } else {
            product_card(ui, false, |ui| {
                ui.label(
                    RichText::new("CURRENT ASSET")
                        .color(VisualFoundryTokens::dark().colors.accent_hover)
                        .small(),
                );
                let _ = self.show_current_preview_sized(ui, 320.0);
            });
            ui.add_space(12.0);
            commands.extend(show_export_readiness_panel(
                ui,
                can_export,
                &pack_view,
                &pack_panel,
            ));
        }
        commands
    }

    pub(super) fn show_pack(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let view = pack::pack_panel_view(&self.state.pack);
        section_header(
            ui,
            SectionHeaderSpec {
                eyebrow: "Pack",
                title: "Pack preview",
                subtitle: Some("Collect coherent variants before exporting a set."),
            },
        );
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let add_enabled = self.state.document.is_some();
            if action_button(
                ui,
                &action_spec(
                    add_enabled,
                    ACTION_ADD_CURRENT_ASSET,
                    ButtonTone::Primary,
                    NEED_PROJECT_REASON,
                ),
            )
            .clicked()
                && let Some(command) = self.add_current_to_pack_command()
            {
                commands.push(command);
            }
            let batch_export_reason = view
                .export
                .disabled_reason
                .as_ref()
                .map(|reason| product_panel_message(reason, NEED_PACK_MEMBER_REASON))
                .unwrap_or_else(|| NEED_PACK_MEMBER_REASON.to_owned());
            if action_button(
                ui,
                &action_spec(
                    view.export.enabled,
                    ACTION_EXPORT_PACK,
                    ButtonTone::Secondary,
                    batch_export_reason.as_str(),
                ),
            )
            .clicked()
                && let Some(out_dir) = select_pack_export_dir()
                && let Some(command) = pack::batch_export_command(&self.state.pack, out_dir)
            {
                commands.push(command);
            }
        });
        ui.add_space(12.0);
        if self.state.document.is_none() && view.members.is_empty() {
            commands.extend(self.show_choose_asset_empty_state(
                ui,
                "Choose an asset first",
                "Choose Box Primitive, Lidded Box, Flat Panel Primitive, Sphere Primitive, Hinged Panel, Panel with Knob, or open a project before starting a pack.",
            ));
            return commands;
        }
        if !view.active {
            ui.columns(2, |columns| {
                product_card(&mut columns[0], false, |ui| {
                    let colors = VisualFoundryTokens::dark().colors;
                    ui.label(
                        RichText::new("CURRENT ASSET")
                            .color(colors.accent_hover)
                            .small(),
                    );
                    let _ = self.show_current_preview_sized(ui, 280.0);
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(self.current_project_title())
                            .color(colors.text)
                            .strong(),
                    );
                    if action_button(
                        ui,
                        &ActionSpec::enabled(ACTION_ADD_CURRENT_ASSET, ButtonTone::Primary),
                    )
                    .clicked()
                        && let Some(command) = self.add_current_to_pack_command()
                    {
                        commands.push(command);
                    }
                });
                product_card(&mut columns[1], false, |ui| {
                    let colors = VisualFoundryTokens::dark().colors;
                    ui.label(RichText::new("Add assets to export").color(colors.text));
                    ui.label(
                        RichText::new("0 assets in pack")
                            .color(colors.text_muted)
                            .small(),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(NEED_PACK_MEMBER_REASON)
                            .color(colors.warning)
                            .small(),
                    );
                });
            });
            return commands;
        }
        ui.columns(2, |columns| {
            let current_preview = self.state.current_preview.as_ref();
            let current_build = self.state.current_build.as_ref();
            let texture_cache = &mut self.texture_cache;
            product_card(&mut columns[0], false, |ui| {
                ui.label(
                    RichText::new("Contact sheet")
                        .color(VisualFoundryTokens::dark().colors.accent_hover)
                        .small(),
                );
                ui.label(
                    RichText::new(format!("{} assets in pack", view.contact_sheet.cells.len()))
                        .color(VisualFoundryTokens::dark().colors.text),
                );
                ui.add_space(6.0);
                show_pack_contact_sheet(
                    ui,
                    texture_cache,
                    current_preview,
                    current_build,
                    &view.contact_sheet,
                );
            });

            product_card(&mut columns[1], view.export.enabled, |ui| {
                let colors = VisualFoundryTokens::dark().colors;
                ui.label(RichText::new(pack_readiness_label(&view)).color(colors.text));
                ui.label(
                    RichText::new(pack_readiness_detail(&view))
                        .color(colors.text_muted)
                        .small(),
                );
                if !view.coherence_warnings.is_empty() {
                    ui.add_space(8.0);
                    for warning in view.coherence_warnings.iter().take(3) {
                        ui.label(
                            RichText::new(product_panel_message(
                                &warning.message,
                                "Pack needs attention before export.",
                            ))
                            .color(colors.warning),
                        );
                    }
                }
            });
        });
        commands
    }

    pub(super) fn show_pack_drawer(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        self.show_pack(ui)
    }

    pub(super) fn show_export_drawer(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        self.show_export(ui)
    }

    pub(super) fn add_current_to_pack_command(&self) -> Option<FoundryAppCommand> {
        let document = self.state.document.as_ref()?;
        let pack_id = self
            .state
            .pack
            .pack_id
            .clone()
            .unwrap_or_else(|| "foundry_pack".to_owned());
        let member_id = unique_pack_member_id(&self.state.pack, &document.document_id.0);
        Some(pack::add_current_asset_to_pack_command(pack_id, member_id))
    }
}
