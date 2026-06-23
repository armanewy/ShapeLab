//! Background job contracts for the native Foundry surface.

use std::path::PathBuf;

use shape_foundry::{
    FoundryAssetDocument, FoundryCompilationOutput, FoundryEdit, FoundryPackDocument,
};
use shape_render::OrbitCamera;
use shape_search::foundry::{
    FoundryCandidateMode, FoundryCandidateOutput, FoundryCandidateRequest,
};

use super::view_model::{FoundryCandidateCard, FoundryPackView};

/// Deterministic background work requested by the Foundry app state.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FoundryJobRequest {
    /// Compile the current semantic source into an exact recipe/artifact.
    CompileCurrent {
        /// Monotonic app-local job ID.
        job_id: u64,
        /// Source document snapshot.
        document: Box<FoundryAssetDocument>,
    },
    /// Render a whole-model preview from an already compiled output.
    RenderPreview {
        /// Monotonic app-local job ID.
        job_id: u64,
        /// Compiled output to render.
        output: Box<FoundryCompilationOutput>,
        /// Requested image width.
        width: u32,
        /// Requested image height.
        height: u32,
    },
    /// Generate candidate directions off the UI thread.
    GenerateCandidates {
        /// Monotonic app-local job ID.
        job_id: u64,
        /// Source document snapshot.
        document: Box<FoundryAssetDocument>,
        /// Deterministic search request, including the search mode.
        request: FoundryCandidateRequest,
    },
    /// Apply a replayable edit and rebuild the resulting document.
    ApplyEdit {
        /// Monotonic app-local job ID.
        job_id: u64,
        /// Parent document snapshot.
        document: Box<FoundryAssetDocument>,
        /// Replayable Foundry edit.
        edit: Box<FoundryEdit>,
    },
    /// Compile a family pack.
    CompilePack {
        /// Monotonic app-local job ID.
        job_id: u64,
        /// Pack snapshot.
        pack: Box<FoundryPackDocument>,
    },
    /// Export an already compiled model package.
    Export {
        /// Monotonic app-local job ID.
        job_id: u64,
        /// Compiled output to export.
        output: Box<FoundryCompilationOutput>,
        /// Export profile key.
        profile: String,
        /// Destination directory.
        out_dir: PathBuf,
    },
}

impl FoundryJobRequest {
    /// Return the app-local job ID.
    #[must_use]
    pub(crate) fn job_id(&self) -> u64 {
        match self {
            Self::CompileCurrent { job_id, .. }
            | Self::RenderPreview { job_id, .. }
            | Self::GenerateCandidates { job_id, .. }
            | Self::ApplyEdit { job_id, .. }
            | Self::CompilePack { job_id, .. }
            | Self::Export { job_id, .. } => *job_id,
        }
    }

    /// Return the candidate mode for candidate-generation work.
    #[must_use]
    pub(crate) fn candidate_mode(&self) -> Option<FoundryCandidateMode> {
        match self {
            Self::GenerateCandidates { request, .. } => Some(request.mode),
            _ => None,
        }
    }
}

/// Result event emitted by a Foundry background worker.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FoundryJobEvent {
    /// Compilation completed.
    CompileFinished {
        /// Matching app-local job ID.
        job_id: u64,
        /// Exact compilation output.
        output: Box<FoundryCompilationOutput>,
    },
    /// A preview image was rendered.
    PreviewRendered {
        /// Matching app-local job ID.
        job_id: u64,
        /// Stable preview slot ID.
        preview_id: String,
        /// RGBA8 image bytes.
        rgba8: Vec<u8>,
        /// Image width.
        width: u32,
        /// Image height.
        height: u32,
        /// Camera used for the preview.
        camera: OrbitCamera,
    },
    /// Candidate search completed.
    CandidatesGenerated {
        /// Matching app-local job ID.
        job_id: u64,
        /// Originating search request, including the search mode.
        request: FoundryCandidateRequest,
        /// Search output and diagnostics.
        output: Box<FoundryCandidateOutput>,
        /// UI-ready candidate cards.
        cards: Vec<FoundryCandidateCard>,
    },
    /// A Foundry edit was applied.
    EditApplied {
        /// Matching app-local job ID.
        job_id: u64,
        /// Replayable edit.
        edit: Box<FoundryEdit>,
        /// Rebuilt output after the edit.
        output: Box<FoundryCompilationOutput>,
    },
    /// Pack compilation completed.
    PackCompiled {
        /// Matching app-local job ID.
        job_id: u64,
        /// UI-ready pack view.
        pack: Box<FoundryPackView>,
    },
    /// Export completed.
    ExportFinished {
        /// Matching app-local job ID.
        job_id: u64,
        /// Export profile key.
        profile: String,
        /// Destination directory.
        out_dir: PathBuf,
    },
    /// A job failed.
    Failed {
        /// Matching app-local job ID.
        job_id: u64,
        /// Human-readable diagnostic.
        message: String,
    },
}

impl FoundryJobEvent {
    /// Return the app-local job ID.
    #[must_use]
    pub(crate) fn job_id(&self) -> u64 {
        match self {
            Self::CompileFinished { job_id, .. }
            | Self::PreviewRendered { job_id, .. }
            | Self::CandidatesGenerated { job_id, .. }
            | Self::EditApplied { job_id, .. }
            | Self::PackCompiled { job_id, .. }
            | Self::ExportFinished { job_id, .. }
            | Self::Failed { job_id, .. } => *job_id,
        }
    }
}
