//! UI-independent commands for the native Foundry surface.

use std::path::PathBuf;

use shape_foundry::{ControlValue, FoundryCandidateId, FoundryCommand, FoundryEdit};
use shape_project::foundry::FoundryProject;
use shape_search::foundry::FoundryCandidateRequest;

use super::jobs::FoundryJobRequest;

/// Native UI intent boundary for Foundry.
///
/// Semantic edits are carried as [`FoundryCommand`] or [`FoundryEdit`] so the
/// app does not create a second command language beside the generic Foundry API.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FoundryAppCommand {
    /// Select a customizer control row.
    SelectControl(Option<String>),
    /// Select a candidate direction card.
    SelectCandidate(Option<FoundryCandidateId>),
    /// Select a family-pack member.
    SelectPackMember(Option<String>),
    /// Run one generic Foundry command.
    RunFoundryCommand(Box<FoundryCommand>),
    /// Run one replayable Foundry edit.
    RunFoundryEdit(Box<FoundryEdit>),
    /// Run an ordered generic Foundry command program.
    RunFoundryCommandProgram {
        /// Human-facing revision label.
        label: String,
        /// Ordered commands to replay through the generic Foundry API.
        commands: Vec<FoundryCommand>,
    },
    /// Request candidate generation with an explicit native search policy.
    RequestCandidates(FoundryCandidateRequest),
    /// Render a non-persistent preview for a control value.
    PreviewControlValue {
        /// Control to sample.
        control_id: String,
        /// Transient value to render.
        value: ControlValue,
    },
    /// Request an exact rebuild for the current document.
    RequestBuild,
    /// Request fresh whole-model preview images for current state.
    RequestPreview,
    /// Persist the current project to its existing path.
    Save,
    /// Persist the current project to a new path.
    SaveAs(PathBuf),
    /// Load a Foundry project file.
    Load(PathBuf),
    /// Validate, compile, and export the current family pack through the reducer job registry.
    RequestPackBatchExport {
        /// Destination directory for member package folders.
        out_dir: PathBuf,
    },
    /// Toggle the Advanced Recipe drawer.
    SetAdvancedRecipeOpen(bool),
}

impl FoundryAppCommand {
    /// Convenience constructor for one generic Foundry command.
    #[must_use]
    pub(crate) fn run(command: FoundryCommand) -> Self {
        Self::RunFoundryCommand(Box::new(command))
    }

    /// Return the wrapped command when this command carries exactly one
    /// generic Foundry command.
    #[must_use]
    pub(crate) fn single_foundry_command(&self) -> Option<&FoundryCommand> {
        match self {
            Self::RunFoundryCommand(command) => Some(command),
            _ => None,
        }
    }
}

/// Side effects requested by the Foundry state reducer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FoundryAppEffect {
    /// Run deterministic background work off the UI thread.
    StartJob(Box<FoundryJobRequest>),
    /// Persist a project snapshot to disk.
    SaveProject {
        /// Destination path.
        path: PathBuf,
        /// Project payload to write.
        project: Box<FoundryProject>,
    },
    /// Load a Foundry project file.
    LoadProject(PathBuf),
}
