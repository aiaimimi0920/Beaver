#[path = "object_attempt_test_fixture.rs"]
mod attempt_fixture;
#[path = "object_task_cancel_tests.rs"]
mod cancel;
#[path = "object_task_cancel_scope_tests.rs"]
mod cancel_scope;
#[path = "object_task_cancel_validation_tests.rs"]
mod cancel_validation;
#[path = "object_task_commit_tests.rs"]
mod commit;
#[path = "object_task_definition_atomic_tests.rs"]
mod definition_atomic;
#[path = "object_task_definition_fixture.rs"]
mod definition_fixture;
#[path = "object_task_definition_validation_tests.rs"]
mod definition_validation;
#[path = "object_task_definition_tests.rs"]
mod definitions;
#[path = "object_task_draft_lock_tests.rs"]
mod draft_lock;
#[path = "object_task_draft_tests.rs"]
mod drafts;
#[path = "object_attempt_callback_tests.rs"]
mod object_attempt_callback;
#[path = "object_attempt_check_tests.rs"]
mod object_attempt_check;
#[path = "object_attempt_check_history_tests.rs"]
mod object_attempt_check_history;
#[path = "object_attempt_claim_tests.rs"]
mod object_attempt_claim;
#[path = "object_attempt_control_tests.rs"]
mod object_attempt_control;
#[path = "object_attempt_control_integrity_tests.rs"]
mod object_attempt_control_integrity;
#[path = "object_attempt_file_tests.rs"]
mod object_attempt_file;
#[cfg(windows)]
#[path = "object_attempt_godot_tests.rs"]
mod object_attempt_godot;
#[path = "object_attempt_launch_tests.rs"]
mod object_attempt_launch;
#[cfg(windows)]
#[path = "object_attempt_lifecycle_tests.rs"]
mod object_attempt_lifecycle;
#[cfg(windows)]
#[path = "object_attempt_rpc_fixture.rs"]
mod object_attempt_rpc_fixture;
#[path = "object_attempt_scene_preview_tests.rs"]
mod object_attempt_scene_preview;
#[cfg(windows)]
#[path = "object_attempt_scheduler_tests.rs"]
mod object_attempt_scheduler;
#[path = "object_attempt_store_tests.rs"]
mod object_attempt_store;
#[path = "object_attempt_trace_tests.rs"]
mod object_attempt_trace;
#[path = "object_candidate_report_version_tests.rs"]
mod object_candidate_report_version;
#[path = "object_candidate_review_tests.rs"]
mod object_candidate_review;
#[path = "object_candidate_rework_tests.rs"]
mod object_candidate_rework;
#[path = "object_publication_tests.rs"]
mod object_publication;
#[path = "object_publication_deferred_tests.rs"]
mod object_publication_deferred;
#[path = "object_publication_followup_tests.rs"]
mod object_publication_followup;
#[path = "object_publication_policy_tests.rs"]
mod object_publication_policy;
#[path = "object_publication_recovery_tests.rs"]
mod object_publication_recovery;
#[path = "object_recovery_disposition_tests.rs"]
mod object_recovery_disposition;
#[path = "object_recovery_resume_tests.rs"]
mod object_recovery_resume;
#[path = "object_recovery_retry_tests.rs"]
mod object_recovery_retry;
#[path = "object_recovery_verification_tests.rs"]
mod object_recovery_verification;
#[path = "object_run_baseline_tests.rs"]
mod object_run_baseline;
#[path = "object_run_integrity_tests.rs"]
mod object_run_integrity;
#[path = "object_run_recovery_tests.rs"]
mod object_run_recovery;
#[path = "object_run_workspace_tests.rs"]
mod object_run_workspace;
#[path = "object_stage_advance_tests.rs"]
mod object_stage_advance;
#[path = "object_task_baseline_tests.rs"]
mod object_task_baseline;
#[path = "object_task_coarse_dispatch_tests.rs"]
mod object_task_coarse_dispatch;
#[path = "object_task_dispatch_tests.rs"]
mod object_task_dispatch;
#[path = "object_task_dispatch_atomic_tests.rs"]
mod object_task_dispatch_atomic;
#[path = "object_task_queue_integrity_tests.rs"]
mod object_task_queue_integrity;
#[path = "object_task_queue_lifecycle_tests.rs"]
mod object_task_queue_lifecycle;
#[path = "object_task_planning_gap_tests.rs"]
mod planning_gaps;
#[path = "object_publication_test_fixture.rs"]
mod publication_fixture;
#[path = "object_task_queue_fixture.rs"]
mod queue_fixture;
#[path = "object_task_queue_reorder_tests.rs"]
mod queue_reorder;
#[path = "object_task_requirement_tests.rs"]
mod requirements;
#[path = "object_task_revision_tests.rs"]
mod revisions;
#[path = "object_run_test_fixture.rs"]
mod run_fixture;
#[path = "object_task_validation_tests.rs"]
mod validation;
#[path = "validation_object_report_tests.rs"]
mod validation_object_report;
