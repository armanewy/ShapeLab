//! Background job request and event contracts.

#![allow(dead_code)]

use std::path::PathBuf;

use shape_core::ShapeDocument;
use shape_mesh::{MeshSettings, TriangleMesh};
use shape_render::{OrbitCamera, RenderSettings, RenderedImage};
use shape_search::{Candidate, SearchRequest};

/// Monotonic job identifier.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct JobId(pub u64);

/// Monotonic candidate generation identifier.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct GenerationId(pub u64);

/// Request executed off the UI thread.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum JobRequest {
    BuildCurrentPreview {
        job_id: JobId,
        document: ShapeDocument,
        mesh_settings: MeshSettings,
        render_settings: RenderSettings,
        camera: Option<OrbitCamera>,
    },
    GenerateCandidates {
        job_id: JobId,
        generation_id: GenerationId,
        document: ShapeDocument,
        request: SearchRequest,
        candidate_mesh_settings: MeshSettings,
        thumbnail_settings: RenderSettings,
        camera: OrbitCamera,
    },
    RenderCurrentCamera {
        job_id: JobId,
        mesh: TriangleMesh,
        camera: OrbitCamera,
        render_settings: RenderSettings,
    },
    ExportCurrent {
        job_id: JobId,
        mesh: TriangleMesh,
        path: PathBuf,
    },
}

/// Candidate preview returned by worker threads.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CandidatePreview {
    pub slot: usize,
    pub candidate: Candidate,
    pub mesh: TriangleMesh,
    pub image: RenderedImage,
}

/// Background phase/progress event.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum JobEvent {
    Started {
        job_id: JobId,
        phase: JobPhase,
    },
    Progress {
        job_id: JobId,
        phase: JobPhase,
        progress: f32,
    },
    CurrentPreviewReady {
        job_id: JobId,
        mesh: TriangleMesh,
        image: RenderedImage,
        camera: OrbitCamera,
    },
    CandidatePreviewReady {
        job_id: JobId,
        generation_id: GenerationId,
        preview: CandidatePreview,
    },
    GenerationComplete {
        job_id: JobId,
        generation_id: GenerationId,
    },
    ExportComplete {
        job_id: JobId,
        path: PathBuf,
    },
    Failed {
        job_id: JobId,
        message: String,
    },
    Cancelled {
        job_id: JobId,
    },
}

/// Worker phase names.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum JobPhase {
    CompileField,
    Mesh,
    Render,
    Search,
    Export,
}
