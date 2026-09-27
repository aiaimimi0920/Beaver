mod automatic_repair;
mod blender_live_preview;
pub(crate) mod blender_preview;
pub mod code;
mod code_report;
pub mod comparison;
pub mod confirmation;
pub mod coordinator;
mod coverage;
pub mod evidence;
pub mod feedback;
mod feedback_context;
pub mod flow;
mod jobs;
mod judgment;
mod live_preview;
mod live_preview_monitor;
mod live_preview_pick;
#[cfg(test)]
mod live_preview_pick_tests;
#[cfg(test)]
mod live_preview_tests;
mod live_preview_worker;
pub mod manifest;
pub mod model;
pub mod object_report;
pub mod operations;
pub(crate) mod preview_frames;
mod preview_selection;
#[cfg(test)]
mod preview_selection_tests;
pub mod release;
pub mod release_display;
pub mod repository;
pub mod requests;
pub mod roaming;
pub mod runner;
pub mod sandbox;
pub mod service;
pub mod settings;
pub mod visual;

#[cfg(test)]
mod approval_tests;
#[cfg(test)]
mod code_tests;
#[cfg(test)]
mod feedback_context_tests;
#[cfg(test)]
mod project_evidence_tests;
#[cfg(test)]
mod release_tests;
#[cfg(test)]
mod task_integration_tests;
#[cfg(test)]
mod test_support;

pub const RUNNER_VERSION: &str = "beaver-validation-1";
pub const GUT_VERSION: &str = "9.4.0";
pub mod task_completion;
pub mod task_control;
pub mod task_gate;
