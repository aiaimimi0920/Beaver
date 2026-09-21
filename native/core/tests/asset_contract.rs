#[path = "asset_contract/agent.rs"]
mod agent;
#[path = "asset_contract/checkpoint_retention.rs"]
mod checkpoint_retention;
#[path = "asset_contract/delivery.rs"]
mod delivery;
#[path = "asset_contract/feedback.rs"]
mod feedback;
#[path = "asset_contract/fixture.rs"]
mod fixture;
#[path = "asset_contract/project_checkpoints.rs"]
mod project_checkpoints;
#[cfg(windows)]
#[path = "asset_contract/project_reference_links.rs"]
mod project_reference_links;
#[path = "asset_contract/project_references.rs"]
mod project_references;
#[path = "asset_contract/references.rs"]
mod references;
#[path = "asset_contract/stages.rs"]
mod stages;
