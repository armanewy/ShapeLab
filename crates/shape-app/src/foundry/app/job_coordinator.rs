use super::*;

#[derive(Default)]
pub(super) struct FoundryJobCoordinator {
    tx: Option<Sender<FoundryJobEvent>>,
    rx: Option<Receiver<FoundryJobEvent>>,
    preview_cache: Arc<Mutex<FoundryPreviewCache>>,
}

impl FoundryJobCoordinator {
    pub(super) fn submit(&mut self, request: FoundryJobRequest) {
        let job_id = request.job_id();
        let tx = self.tx().clone();
        let preview_cache = Arc::clone(&self.preview_cache);
        thread::spawn(move || {
            let resolver = BuiltInFoundryCatalogResolver::default();
            let event = if request_uses_preview_cache(&request) {
                match preview_cache.lock() {
                    Ok(mut preview_cache) => {
                        run_foundry_job(request, &resolver, &mut preview_cache)
                    }
                    Err(_) => FoundryJobEvent::Failed {
                        job_id,
                        message: "Foundry preview cache lock was poisoned.".to_owned(),
                    },
                }
            } else {
                let mut preview_cache = FoundryPreviewCache::default();
                run_foundry_job(request, &resolver, &mut preview_cache)
            };
            let _ = tx.send(event);
        });
    }

    pub(super) fn try_recv(&mut self) -> Result<FoundryJobEvent, crossbeam_channel::TryRecvError> {
        self.rx().try_recv()
    }

    pub(super) fn reset(&mut self) {
        let (tx, rx) = unbounded();
        self.tx = Some(tx);
        self.rx = Some(rx);
        self.preview_cache = Arc::new(Mutex::new(FoundryPreviewCache::default()));
    }

    pub(super) fn tx(&mut self) -> &Sender<FoundryJobEvent> {
        if self.tx.is_none() || self.rx.is_none() {
            self.reset();
        }
        self.tx.as_ref().expect("tx initialized")
    }

    pub(super) fn rx(&mut self) -> &Receiver<FoundryJobEvent> {
        if self.tx.is_none() || self.rx.is_none() {
            self.reset();
        }
        self.rx.as_ref().expect("rx initialized")
    }
}

pub(super) fn request_uses_preview_cache(request: &FoundryJobRequest) -> bool {
    matches!(
        request,
        FoundryJobRequest::RenderPreview { .. }
            | FoundryJobRequest::PreviewControlValue { .. }
            | FoundryJobRequest::GenerateCandidates { .. }
            | FoundryJobRequest::RenderCandidatePreviews { .. }
    )
}
