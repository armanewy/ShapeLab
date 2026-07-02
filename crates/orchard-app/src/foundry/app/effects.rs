use super::*;

impl FoundryDesktopApp {
    pub(super) fn apply_commands(&mut self, commands: Vec<FoundryAppCommand>, ctx: &egui::Context) {
        let mut state_changed = false;
        for command in commands {
            self.refresh_make_trace_clock();
            if matches!(
                command,
                FoundryAppCommand::RetryPreparation
                    | FoundryAppCommand::RequestBuild
                    | FoundryAppCommand::RequestPreview { .. }
            ) {
                self.make_preparation_started_at = Some(Instant::now());
            }
            if matches!(command, FoundryAppCommand::RequestCandidates(_)) {
                self.make_generation_started_at = Some(Instant::now());
            } else if matches!(command, FoundryAppCommand::CancelIdeaGeneration) {
                self.make_generation_started_at = None;
            }
            match self.state.handle_command(command) {
                Ok(effects) => {
                    state_changed = true;
                    self.apply_effects(effects, ctx);
                }
                Err(error) => self.state.status = Some(error.to_string()),
            }
        }
        if state_changed {
            self.texture_cache.clear();
        }
    }

    pub(super) fn apply_effects(&mut self, effects: Vec<FoundryAppEffect>, ctx: &egui::Context) {
        let mut trace_changed = false;
        for effect in effects {
            match effect {
                FoundryAppEffect::StartJob(request) => {
                    trace_changed = true;
                    self.submit_job(*request);
                }
                FoundryAppEffect::SaveProject { path, project } => {
                    self.save_project(path, *project);
                }
                FoundryAppEffect::LoadProject(path) => self.load_project(path, ctx),
            }
        }
        if trace_changed {
            self.persist_make_job_trace_outputs();
        }
    }

    pub(super) fn submit_job(&mut self, request: FoundryJobRequest) {
        self.jobs.submit(request);
    }

    pub(super) fn poll_jobs(&mut self, ctx: &egui::Context) {
        if self.screenshot_scenario_holds_active_job_capture() {
            ctx.request_repaint_after(Duration::from_millis(250));
            return;
        }

        let mut affected = false;
        let mut trace_changed = false;
        let mut schedule_preview = false;
        let mut schedule_candidate_previews = None;

        loop {
            let event = match self.jobs.try_recv() {
                Ok(event) => event,
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    self.state.status = Some("Foundry background worker disconnected.".to_owned());
                    break;
                }
            };
            self.refresh_make_trace_clock();
            let should_preview = matches!(
                event,
                FoundryJobEvent::CompileFinished { .. } | FoundryJobEvent::EditApplied { .. }
            );
            let candidate_preview_request = match &event {
                FoundryJobEvent::CandidatesGenerated {
                    request, output, ..
                } => Some((request.clone(), output.as_ref().clone())),
                _ => None,
            };
            if self.state.handle_job_event(event) {
                affected = true;
                trace_changed = true;
                schedule_preview |= should_preview;
                if let Some(request) = candidate_preview_request {
                    schedule_candidate_previews = Some(request);
                }
            } else {
                trace_changed = true;
            }
        }

        if schedule_preview {
            self.refresh_make_trace_clock();
            let preview_pixels = current_preview_pixels_for_context(ctx);
            match self
                .state
                .request_preview(preview_pixels, preview_pixels, None)
            {
                Ok(effects) => self.apply_effects(effects, ctx),
                Err(error) => self.state.status = Some(error.to_string()),
            }
        }
        if let Some((request, output)) = schedule_candidate_previews {
            self.refresh_make_trace_clock();
            match self.state.request_candidate_previews(request, output) {
                Ok(effects) => self.apply_effects(effects, ctx),
                Err(error) => self.state.status = Some(error.to_string()),
            }
        }
        if trace_changed {
            self.persist_make_job_trace_outputs();
        }
        if affected {
            self.texture_cache.clear();
            ctx.request_repaint();
        }
    }

    pub(super) fn poll_home_thumbnail_jobs(&mut self, ctx: &egui::Context) {
        if self.home_thumbnails.poll() {
            ctx.request_repaint();
        }
    }
}
