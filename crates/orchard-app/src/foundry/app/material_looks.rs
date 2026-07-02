use super::*;

pub(super) fn load_material_look_evidence(
    report_path: &Path,
    current_artifact_fingerprint: Option<&str>,
) -> Result<MakeMaterialLookEvidence, String> {
    if !report_path.is_file() {
        return Err(MATERIAL_LOOK_MISSING_MESSAGE.to_owned());
    }
    let variants_dir = report_path
        .parent()
        .ok_or_else(|| "Material look report path is invalid.".to_owned())?;
    let package_root = package_root_for_surface_candidate_report(report_path)
        .ok_or_else(|| "Material look package path is invalid.".to_owned())?;
    let report: SurfaceCandidateEvidenceReportFile = read_json_file(report_path)?;
    if report.schema_version != 1 {
        return Err("Material look report schema is not supported.".to_owned());
    }
    if report.profile_id != BOX_PRIMITIVE_PROFILE_ID {
        return Err("Material looks are not part of the Box Primitive baseline.".to_owned());
    }
    if report.visual_foundry_surface_mode_enabled {
        return Err("Material look report overclaims Surface mode readiness.".to_owned());
    }
    if !report.all_candidates_valid || report.candidate_count != MATERIAL_LOOK_TITLES.len() {
        return Err("Material look report does not contain six valid candidates.".to_owned());
    }
    if report.full_ready_status != "blocked" {
        return Err("Material look package must keep full game-ready status blocked.".to_owned());
    }
    if !full_ready_blockers_are_honest(&report.full_ready_blocker_codes) {
        return Err("Material look package is missing required game-ready blockers.".to_owned());
    }

    let candidate_set: SurfaceCandidateSetFile =
        read_json_file(&variants_dir.join(SURFACE_CANDIDATE_SET_FILE))?;
    if candidate_set.schema_version == 0 || candidate_set.profile_id != BOX_PRIMITIVE_PROFILE_ID {
        return Err("Material look candidate set is not for Box Primitive.".to_owned());
    }
    let candidate_rows = candidate_set
        .candidates
        .into_iter()
        .map(|candidate| (candidate.candidate_id.clone(), candidate))
        .collect::<BTreeMap<_, _>>();

    let mut candidates = Vec::with_capacity(report.candidates.len());
    for (index, row) in report.candidates.iter().enumerate() {
        let Some(expected_title) = MATERIAL_LOOK_TITLES.get(index) else {
            return Err("Material look report contains unexpected candidates.".to_owned());
        };
        if row.display_name != *expected_title {
            return Err("Material look candidate titles do not match the approved set.".to_owned());
        }
        if row.shape_delta_leak_detected {
            return Err("Material look evidence detected a shape change.".to_owned());
        }
        if row.result_class == "unsupported" || row.result_class == "duplicate_looking" {
            return Err("Material look evidence is not visually usable.".to_owned());
        }
        let set_row = candidate_rows.get(&row.candidate_id).ok_or_else(|| {
            format!(
                "Material look candidate {} is missing from the candidate set.",
                row.display_name
            )
        })?;
        validate_matching_candidate_refs(row, set_row)?;
        if !package_root.join(&row.material_override_ref).is_file() {
            return Err("Material look material override evidence is missing.".to_owned());
        }
        if !set_row.preserves_frozen_geometry {
            return Err("Material look candidate does not preserve frozen geometry.".to_owned());
        }
        if set_row.full_ready_status != "blocked" || !set_row.blocked_full_ready {
            return Err(
                "Material look candidate must remain blocked from full game-ready.".to_owned(),
            );
        }
        if let Some(fingerprint) = current_artifact_fingerprint
            && set_row.frozen_mesh_fingerprint != fingerprint
        {
            return Err("Material looks do not match this box build.".to_owned());
        }

        let validation_path = package_root.join(&row.validation_ref);
        let validation: SurfaceCandidateValidationFile = read_json_file(&validation_path)?;
        if !validation.valid || !validation.blocker_codes.is_empty() {
            return Err("Material look candidate validation did not pass.".to_owned());
        }
        let delta_path = package_root.join(&row.surface_delta_ref);
        let delta: SurfaceCandidateDeltaFile = read_json_file(&delta_path)?;
        if delta.profile_id != BOX_PRIMITIVE_PROFILE_ID
            || delta.candidate_id != row.candidate_id
            || delta.shape_delta_leak_detected
            || delta.result_class == "unsupported"
        {
            return Err("Material look surface delta is not material-only.".to_owned());
        }

        let preview_path = package_root.join(&row.textured_preview_ref);
        let preview_bytes = fs::read(&preview_path).map_err(|error| {
            format!(
                "Material look textured preview is missing: {} ({error})",
                preview_path.display()
            )
        })?;
        let preview = image::load_from_memory(&preview_bytes)
            .map_err(|error| format!("Material look textured preview could not load: {error}"))?
            .to_rgba8();
        let width = preview.width();
        let height = preview.height();
        if width == 0 || height == 0 {
            return Err("Material look textured preview is empty.".to_owned());
        }

        candidates.push(MakeMaterialLookCandidate {
            candidate_id: row.candidate_id.clone(),
            display_name: row.display_name.clone(),
            material_override_ref: row.material_override_ref.clone(),
            textured_preview_ref: row.textured_preview_ref.clone(),
            surface_delta_ref: row.surface_delta_ref.clone(),
            validation_ref: row.validation_ref.clone(),
            changed_material_slots: set_row.changed_material_slots.clone(),
            rgba8: preview.into_raw(),
            width,
            height,
            visible_surface_pixel_delta: row.visible_surface_pixel_delta,
        });
    }

    Ok(MakeMaterialLookEvidence {
        candidates,
        full_ready_blocker_codes: report.full_ready_blocker_codes,
    })
}

pub(super) fn package_root_for_surface_candidate_report(report_path: &Path) -> Option<PathBuf> {
    let variants_dir = report_path.parent()?;
    let surface_dir = variants_dir.parent()?;
    surface_dir.parent().map(Path::to_path_buf)
}

pub(super) fn read_json_file<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("Could not parse {}: {error}", path.display()))
}

pub(super) fn full_ready_blockers_are_honest(blockers: &[String]) -> bool {
    [
        "manual_review_pending",
        "engine_import_proof_missing",
        "engine_native_package_not_implemented",
        "surface_manual_review_required",
    ]
    .into_iter()
    .all(|required| blockers.iter().any(|blocker| blocker == required))
}

pub(super) fn validate_matching_candidate_refs(
    report: &SurfaceCandidateEvidenceReportRowFile,
    candidate: &SurfaceCandidateSetRowFile,
) -> Result<(), String> {
    if report.display_name != candidate.display_name
        || report.material_override_ref != candidate.material_override_ref
        || report.textured_preview_ref != candidate.textured_preview_ref
        || report.surface_delta_ref != candidate.surface_delta_ref
        || report.validation_ref != candidate.validation_ref
    {
        Err("Material look report and candidate set disagree.".to_owned())
    } else if report.material_override_ref.trim().is_empty()
        || report.textured_preview_ref.trim().is_empty()
        || report.surface_delta_ref.trim().is_empty()
        || report.validation_ref.trim().is_empty()
    {
        Err("Material look evidence has missing file references.".to_owned())
    } else {
        Ok(())
    }
}

pub(super) fn show_material_look_comparison_card(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_preview: Option<&FoundryPreviewImage>,
    current_build: Option<&FoundryBuildStamp>,
    selected_candidate: &MakeMaterialLookCandidate,
) {
    product_card(ui, true, |ui| {
        let colors = VisualFoundryTokens::dark().colors;
        ui.horizontal_wrapped(|ui| {
            let _ = status_pill(
                ui,
                StatusPillSpec::new(MATERIAL_LOOK_SURFACE_ONLY_COPY, StatusTone::Ready),
            );
            let _ = status_pill(
                ui,
                StatusPillSpec::new(MATERIAL_LOOK_GEOMETRY_UNCHANGED_COPY, StatusTone::Ready),
            );
        });
        ui.add_space(6.0);
        let preview_edge = (ui.available_width() * 0.22).clamp(88.0, 132.0);
        ui.horizontal_top(|ui| {
            ui.vertical_centered(|ui| {
                ui.set_width(preview_edge + 24.0);
                ui.label(
                    RichText::new("Current Material")
                        .color(colors.text)
                        .strong(),
                );
                if let Some(preview) = current_preview {
                    show_rgba_preview(
                        ui,
                        texture_cache,
                        FoundryPreviewDraw {
                            preview_id: "material-look-current-comparison",
                            build: preview.build.as_ref(),
                            rgba8: &preview.rgba8,
                            width: preview.width,
                            height: preview.height,
                            max_edge: preview_edge,
                        },
                    );
                } else {
                    ui.label(RichText::new("Preview pending").color(colors.text_muted));
                }
            });
            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                ui.set_width(preview_edge + 24.0);
                ui.label(
                    RichText::new("Candidate Material")
                        .color(colors.text)
                        .strong(),
                );
                let preview_id = format!(
                    "material-look-selected-comparison-{}",
                    selected_candidate.candidate_id
                );
                show_rgba_preview(
                    ui,
                    texture_cache,
                    FoundryPreviewDraw {
                        preview_id: &preview_id,
                        build: current_build,
                        rgba8: &selected_candidate.rgba8,
                        width: selected_candidate.width,
                        height: selected_candidate.height,
                        max_edge: preview_edge,
                    },
                );
            });
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new(&selected_candidate.display_name)
                .color(colors.text)
                .strong(),
        );
        ui.add(
            egui::Label::new(
                RichText::new(material_look_changed_summary(selected_candidate))
                    .color(colors.text_muted)
                    .small(),
            )
            .wrap(),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new(MATERIAL_LOOK_PREVIEW_ONLY_COPY)
                .color(colors.warning)
                .small(),
        );
        ui.label(
            RichText::new(MATERIAL_LOOK_FULL_READY_BLOCKED_COPY)
                .color(colors.text_muted)
                .small(),
        );
    });
}

pub(super) fn show_material_look_candidate_grid(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_build: Option<&FoundryBuildStamp>,
    candidates: &[MakeMaterialLookCandidate],
    selected_candidate_id: &str,
    compact: bool,
) -> Option<String> {
    let mut selected = None;
    let columns = if compact {
        if ui.available_width() >= 760.0 { 3 } else { 2 }
    } else if ui.available_width() >= 760.0 {
        3
    } else {
        2
    };
    let preview_edge = if compact { 96.0 } else { 156.0 };
    for row in candidates.chunks(columns) {
        ui.columns(row.len(), |uis| {
            for (column, candidate) in uis.iter_mut().zip(row) {
                let is_selected = candidate.candidate_id == selected_candidate_id;
                product_card(column, is_selected, |ui| {
                    let colors = VisualFoundryTokens::dark().colors;
                    ui.label(
                        RichText::new(&candidate.display_name)
                            .color(colors.text)
                            .strong(),
                    );
                    ui.add_space(6.0);
                    let preview_id = format!("material-look-card-{}", candidate.candidate_id);
                    show_rgba_preview(
                        ui,
                        texture_cache,
                        FoundryPreviewDraw {
                            preview_id: &preview_id,
                            build: current_build,
                            rgba8: &candidate.rgba8,
                            width: candidate.width,
                            height: candidate.height,
                            max_edge: preview_edge,
                        },
                    );
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        let _ = status_pill(
                            ui,
                            StatusPillSpec::new(MATERIAL_LOOK_SURFACE_ONLY_COPY, StatusTone::Ready),
                        );
                        let _ = status_pill(
                            ui,
                            StatusPillSpec::new(
                                MATERIAL_LOOK_GEOMETRY_UNCHANGED_COPY,
                                StatusTone::Ready,
                            ),
                        );
                    });
                    if !compact {
                        ui.add_space(6.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(material_look_changed_summary(candidate))
                                    .color(colors.text_muted)
                                    .small(),
                            )
                            .wrap(),
                        );
                        if let Some(delta) = candidate.visible_surface_pixel_delta {
                            ui.label(
                                RichText::new(format!(
                                    "Visible material difference {:.1}%",
                                    delta * 100.0
                                ))
                                .color(colors.text_subtle)
                                .small(),
                            );
                        }
                    }
                    ui.add_space(8.0);
                    if action_button(
                        ui,
                        &ActionSpec::enabled(ACTION_SELECT, ButtonTone::Secondary),
                    )
                    .clicked()
                    {
                        selected = Some(candidate.candidate_id.clone());
                    }
                });
            }
        });
        ui.add_space(8.0);
    }
    selected
}

pub(super) fn material_look_changed_summary(candidate: &MakeMaterialLookCandidate) -> String {
    let labels = candidate
        .changed_material_slots
        .iter()
        .filter_map(|slot| material_look_slot_summary_label(slot))
        .collect::<BTreeSet<_>>();
    if labels.is_empty() {
        "Changes the visible finish while keeping the box shape fixed.".to_owned()
    } else {
        format!(
            "Changes {} while keeping the box shape fixed.",
            human_join(labels.into_iter().collect::<Vec<_>>().as_slice())
        )
    }
}

pub(super) fn material_look_slot_summary_label(slot: &str) -> Option<&'static str> {
    match slot {
        "painted_metal_body" => Some("body finish"),
        "shadowed_body_edges" => Some("edge contrast"),
        "exposed_edge_detail" => Some("edge detail"),
        "soft_edge_highlights" => Some("edge highlights"),
        "fallback_hard_surface" => Some("secondary surfaces"),
        _ => None,
    }
}

pub(super) fn human_join(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let mut joined = items[..items.len() - 1].join(", ");
            joined.push_str(", and ");
            joined.push_str(items[items.len() - 1]);
            joined
        }
    }
}

impl FoundryDesktopApp {
    pub(super) fn material_look_action_visible(&self, view_state: &MakeCanvasViewState) -> bool {
        let _ = view_state;
        false
    }

    pub(super) fn material_look_report_path(&self) -> PathBuf {
        self.material_looks
            .evidence_report_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(SURFACE_CANDIDATE_REPORT_RELATIVE_PATH))
    }

    pub(super) fn current_artifact_fingerprint_hex(&self) -> Option<String> {
        self.state
            .current_build
            .as_ref()
            .map(|build| build.artifact_fingerprint.0.to_hex())
    }

    pub(super) fn open_material_looks_panel(&mut self) {
        self.material_looks.tray_open = true;
        self.material_looks.load_error = None;

        let Some(current_fingerprint) = self.current_artifact_fingerprint_hex() else {
            self.material_looks.load_error =
                Some("Prepare the Box Primitive before trying material looks.".to_owned());
            self.material_looks.evidence = None;
            self.material_looks.selected_candidate_id = None;
            return;
        };

        let report_path = self.material_look_report_path();
        match load_material_look_evidence(&report_path, Some(current_fingerprint.as_str())) {
            Ok(evidence) => {
                if self
                    .material_looks
                    .selected_candidate_id
                    .as_ref()
                    .is_none_or(|selected| {
                        !evidence
                            .candidates
                            .iter()
                            .any(|candidate| &candidate.candidate_id == selected)
                    })
                {
                    self.material_looks.selected_candidate_id = evidence
                        .candidates
                        .first()
                        .map(|candidate| candidate.candidate_id.clone());
                }
                self.material_looks.evidence = Some(evidence);
            }
            Err(error) => {
                self.material_looks.load_error = Some(error);
                self.material_looks.evidence = None;
                self.material_looks.selected_candidate_id = None;
            }
        }
    }

    pub(super) fn selected_material_look(&self) -> Option<&MakeMaterialLookCandidate> {
        let evidence = self.material_looks.evidence.as_ref()?;
        let selected_id = self.material_looks.selected_candidate_id.as_deref();
        selected_id
            .and_then(|id| {
                evidence
                    .candidates
                    .iter()
                    .find(|candidate| candidate.candidate_id == id)
            })
            .or_else(|| evidence.candidates.first())
    }

    pub(super) fn material_look_export_copy(&self) -> Option<(&'static str, &'static str)> {
        if self.active_make_profile_kind().simple_clay_make_baseline() {
            return None;
        }
        self.selected_material_look().map(|_| {
            (
                MATERIAL_LOOK_PREVIEW_ONLY_COPY,
                MATERIAL_LOOK_FULL_READY_BLOCKED_COPY,
            )
        })
    }

    pub(super) fn current_export_copy(&self) -> (&'static str, &'static str) {
        if self.active_make_profile_kind().is_lidded_box() {
            (LIDDED_BOX_EXPORT_TITLE, LIDDED_BOX_EXPORT_DETAIL)
        } else if self.active_make_profile_kind().is_flat_panel_primitive() {
            (FLAT_PANEL_EXPORT_TITLE, FLAT_PANEL_EXPORT_DETAIL)
        } else if self.active_make_profile_kind().is_sphere_primitive() {
            (
                SPHERE_PRIMITIVE_EXPORT_TITLE,
                SPHERE_PRIMITIVE_EXPORT_DETAIL,
            )
        } else if self.active_make_profile_kind().is_hinged_panel() {
            (HINGED_PANEL_EXPORT_TITLE, HINGED_PANEL_EXPORT_DETAIL)
        } else if self.active_make_profile_kind().is_handled_panel() {
            (HANDLED_PANEL_EXPORT_TITLE, HANDLED_PANEL_EXPORT_DETAIL)
        } else if self.active_make_profile_kind().is_panel_with_knob() {
            (PANEL_KNOB_EXPORT_TITLE, PANEL_KNOB_EXPORT_DETAIL)
        } else {
            (BOX_PRIMITIVE_EXPORT_TITLE, BOX_PRIMITIVE_EXPORT_DETAIL)
        }
    }

    pub(super) fn show_material_look_inspector_summary(&mut self, ui: &mut egui::Ui) {
        let colors = VisualFoundryTokens::dark().colors;
        compact_section_header(
            ui,
            MATERIAL_LOOK_SURFACE_ONLY_COPY,
            MATERIAL_LOOK_SECTION_TITLE,
            MATERIAL_LOOK_GEOMETRY_UNCHANGED_COPY,
        );
        ui.add_space(6.0);

        if let Some(error) = self.material_looks.load_error.clone() {
            ui.label(
                RichText::new(if error == MATERIAL_LOOK_MISSING_MESSAGE {
                    MATERIAL_LOOK_MISSING_MESSAGE
                } else {
                    "Material looks unavailable"
                })
                .color(colors.text)
                .strong(),
            );
            ui.label(
                RichText::new(if error == MATERIAL_LOOK_MISSING_MESSAGE {
                    SURFACE_PACKAGE_COMMAND_COPY
                } else {
                    error.as_str()
                })
                .color(colors.text_muted)
                .small(),
            );
            return;
        }

        let Some(evidence) = self.material_looks.evidence.clone() else {
            ui.label(
                RichText::new(MATERIAL_LOOK_MISSING_MESSAGE)
                    .color(colors.text_muted)
                    .small(),
            );
            return;
        };
        if evidence.candidates.is_empty() {
            ui.label(
                RichText::new("Material looks unavailable")
                    .color(colors.text_muted)
                    .small(),
            );
            return;
        }

        let selected_id = self
            .material_looks
            .selected_candidate_id
            .as_deref()
            .unwrap_or_else(|| evidence.candidates[0].candidate_id.as_str());
        let selected_candidate = evidence
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == selected_id)
            .unwrap_or(&evidence.candidates[0])
            .clone();
        let current_preview = self.state.current_preview.clone();
        let current_build = self.state.current_build.clone();
        let texture_cache = &mut self.texture_cache;
        let preview_edge = (ui.available_width() * 0.18).clamp(54.0, 72.0);

        ui.horizontal_top(|ui| {
            ui.vertical_centered(|ui| {
                ui.set_width((ui.available_width() * 0.40).max(120.0));
                ui.label(
                    RichText::new("Current Material")
                        .color(colors.text)
                        .small()
                        .strong(),
                );
                if let Some(preview) = current_preview.as_ref() {
                    show_rgba_preview(
                        ui,
                        texture_cache,
                        FoundryPreviewDraw {
                            preview_id: "material-look-inspector-current",
                            build: preview.build.as_ref(),
                            rgba8: &preview.rgba8,
                            width: preview.width,
                            height: preview.height,
                            max_edge: preview_edge,
                        },
                    );
                }
            });
            ui.vertical_centered(|ui| {
                ui.set_width((ui.available_width() * 0.55).max(120.0));
                ui.label(
                    RichText::new("Candidate Material")
                        .color(colors.text)
                        .small()
                        .strong(),
                );
                let preview_id = format!(
                    "material-look-inspector-selected-{}",
                    selected_candidate.candidate_id
                );
                show_rgba_preview(
                    ui,
                    texture_cache,
                    FoundryPreviewDraw {
                        preview_id: &preview_id,
                        build: current_build.as_ref(),
                        rgba8: &selected_candidate.rgba8,
                        width: selected_candidate.width,
                        height: selected_candidate.height,
                        max_edge: preview_edge,
                    },
                );
            });
        });
        ui.label(
            RichText::new(&selected_candidate.display_name)
                .color(colors.text)
                .small()
                .strong(),
        );
        ui.label(
            RichText::new(MATERIAL_LOOK_PREVIEW_ONLY_COPY)
                .color(colors.warning)
                .small(),
        );
        ui.add_space(6.0);

        let columns = evidence.candidates.iter().take(3).collect::<Vec<_>>();
        if !columns.is_empty() {
            let column_width = ((ui.available_width() - 12.0) / columns.len() as f32).max(84.0);
            let mut selected = None;
            ui.horizontal_top(|ui| {
                for candidate in columns {
                    ui.allocate_ui_with_layout(
                        egui::vec2(column_width, 132.0),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            let preview_id =
                                format!("material-look-inspector-card-{}", candidate.candidate_id);
                            show_rgba_preview(
                                ui,
                                texture_cache,
                                FoundryPreviewDraw {
                                    preview_id: &preview_id,
                                    build: current_build.as_ref(),
                                    rgba8: &candidate.rgba8,
                                    width: candidate.width,
                                    height: candidate.height,
                                    max_edge: 54.0,
                                },
                            );
                            ui.label(
                                RichText::new(&candidate.display_name)
                                    .color(colors.text)
                                    .small(),
                            );
                            if action_button(
                                ui,
                                &ActionSpec::enabled(ACTION_SELECT, ButtonTone::Secondary),
                            )
                            .clicked()
                            {
                                selected = Some(candidate.candidate_id.clone());
                            }
                        },
                    );
                }
            });
            if let Some(candidate_id) = selected {
                self.material_looks.selected_candidate_id = Some(candidate_id);
            }
        }
    }

    pub(super) fn show_material_looks_panel(&mut self, ui: &mut egui::Ui) {
        compact_section_header(
            ui,
            MATERIAL_LOOK_SURFACE_ONLY_COPY,
            MATERIAL_LOOK_SECTION_TITLE,
            MATERIAL_LOOK_GEOMETRY_UNCHANGED_COPY,
        );
        ui.add_space(8.0);

        if let Some(error) = self.material_looks.load_error.clone() {
            let title = if error == MATERIAL_LOOK_MISSING_MESSAGE {
                MATERIAL_LOOK_MISSING_MESSAGE
            } else {
                "Material looks unavailable"
            };
            product_card(ui, false, |ui| {
                let colors = VisualFoundryTokens::dark().colors;
                ui.label(RichText::new(title).color(colors.text).strong());
                ui.add(
                    egui::Label::new(
                        RichText::new(if error == MATERIAL_LOOK_MISSING_MESSAGE {
                            "Material looks are not part of the Box Primitive baseline."
                        } else {
                            error.as_str()
                        })
                        .color(colors.text_muted),
                    )
                    .wrap(),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(SURFACE_PACKAGE_COMMAND_COPY)
                        .color(colors.accent_hover)
                        .small()
                        .strong(),
                );
                ui.monospace(SURFACE_PACKAGE_COMMAND);
            });
            return;
        }

        let Some(evidence) = self.material_looks.evidence.clone() else {
            product_compact_empty_state(
                ui,
                MATERIAL_LOOK_MISSING_MESSAGE,
                "Material looks are not part of the Box Primitive baseline.",
            );
            return;
        };
        if evidence.candidates.is_empty() {
            product_compact_empty_state(
                ui,
                "Material looks unavailable",
                "The surface package did not include valid material candidates.",
            );
            return;
        }

        let selected_id = self
            .material_looks
            .selected_candidate_id
            .as_deref()
            .unwrap_or_else(|| evidence.candidates[0].candidate_id.as_str());
        let selected_candidate = evidence
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == selected_id)
            .unwrap_or(&evidence.candidates[0])
            .clone();
        if !evidence.full_ready_blocker_codes.is_empty() {
            ui.label(
                RichText::new(MATERIAL_LOOK_FULL_READY_BLOCKED_COPY)
                    .color(VisualFoundryTokens::dark().colors.text_muted)
                    .small(),
            );
            ui.add_space(6.0);
        }
        let current_preview = self.state.current_preview.as_ref();
        let current_build = self.state.current_build.as_ref();
        let texture_cache = &mut self.texture_cache;
        let panel_width = ui.available_width();
        let selected = if panel_width >= 980.0 {
            let comparison_width = (panel_width * 0.38).clamp(360.0, 580.0);
            let mut selected = None;
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(comparison_width, ui.available_height().max(280.0)),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        show_material_look_comparison_card(
                            ui,
                            texture_cache,
                            current_preview,
                            current_build,
                            &selected_candidate,
                        );
                    },
                );
                ui.add_space(10.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), ui.available_height().max(280.0)),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        selected = show_material_look_candidate_grid(
                            ui,
                            texture_cache,
                            current_build,
                            &evidence.candidates,
                            selected_candidate.candidate_id.as_str(),
                            true,
                        );
                    },
                );
            });
            selected
        } else {
            show_material_look_comparison_card(
                ui,
                texture_cache,
                current_preview,
                current_build,
                &selected_candidate,
            );
            ui.add_space(8.0);
            show_material_look_candidate_grid(
                ui,
                texture_cache,
                current_build,
                &evidence.candidates,
                selected_candidate.candidate_id.as_str(),
                false,
            )
        };

        if let Some(selected) = selected {
            self.material_looks.selected_candidate_id = Some(selected);
        }
    }
}
