mod automatic_repair;
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
pub mod manifest;
pub mod model;
pub mod operations;
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
