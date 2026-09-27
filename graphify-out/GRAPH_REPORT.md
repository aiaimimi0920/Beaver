# Graph Report - Beaver  (2026-09-14)

## Corpus Check
- Large corpus: 536 files · ~273,387 words. Semantic extraction will be expensive (many Claude tokens). Consider running on a subfolder.

## Summary
- 3446 nodes · 7867 edges · 193 communities (173 shown, 20 thin omitted)
- Extraction: 97% EXTRACTED · 3% INFERRED · 0% AMBIGUOUS · INFERRED: 209 edges (avg confidence: 0.78)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `958fc2cd`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Shared Game Design
- Electron Core Export Templates
- Prototypes Prototype
- Tests Recovery Test
- Electron Core Settings
- Electron Core Files
- Electron Core Executor
- UI Api
- Shared Autonomy
- UI Studio Preview
- UI Notification State
- UI Feedback Draft
- Scripts Native Release
- UI Preview Controls
- Electron Core Tasks
- Electron Core Tool Setup
- Electron Core Projects
- UI Validation Types
- Prototypes Prototype
- Resources Asset Observer Transport
- Shared Engine Packages
- Scripts Build Native
- Scripts Native Business Smoke
- UI Documents View
- Scripts Native Media Smoke
- Scripts Native Performance Smoke
- Scripts Vendor Npr
- Prototypes Prototype
- Prototypes Prototype
- Scripts Package
- Prototypes App
- Scripts Desktop Smoke
- Native Asset Task
- Native Preferences
- Desktop Business Api
- Native Capture
- Native Launch
- Native Reveal
- Native Scheduler
- Native Task Resources
- Native Release Tests
- Desktop Desktop Schema
- Desktop Business Mcp
- Native Migration Import Tests
- Native Scheduler
- Native Task Brief
- Native Settings
- Desktop Task Control
- Native Executor Tools
- Native Feedback Context
- Native Approval Tests
- Desktop Desktop Schema
- Desktop Desktop Schema
- Desktop Windows Schema
- Native Flow
- Desktop Business Catalog
- Desktop Template Runtime
- Native Asset Tool
- Desktop Desktop Schema
- Desktop Windows Schema
- Desktop Asset Task Runtime
- Desktop Asset Task Windows
- Desktop Shell
- Native Asset Withdrawal
- Native Coverage
- Desktop Business Routing
- Desktop Task Runtime
- Native Rpc
- Native Executor Tools Tests
- Native Export Receipt
- Native Source Encoding
- Native Confirmation
- Native Coordinator
- Native Evidence
- Native Migration Bundle
- Native Runner
- Desktop Windows Schema
- Desktop Game Runtime
- Scripts Icon Review Smoke
- Native Rpc Handshake
- Desktop Workflow Runtime
- Native Automatic Repair
- Scripts Verify Release
- Native Executor Contract
- Native Rpc Contract
- Desktop Validation Catalog
- Native Brief Contract
- Native Data Backup
- Native Export Contract
- Native Files Contract
- Native Migrated Session
- Native Migration Bundle
- Native Process Contract
- Native Project Contract
- Native Scheduler Contract
- Native Store Contract
- Native Tool Probe Fixture
- Native Vault Contract
- Native Workflow Contract
- Native Blender Session
- Native Feedback Context Tests
- Desktop Asset Task Catalog
- Native Cargo
- Scripts Build
- Native Tests Fixture
- Native Capture Windows
- Native Game Play
- Native Store
- Desktop Backend
- Native Journal
- Native Game Export
- Native Validation Model
- Code Package
- Desktop Business Api
- Native Media
- Native Assets
- Native Tool Setup
- Desktop Asset Runtime
- Native Files
- Native Executor
- Native Policy
- Native Task Actions
- Native Task Create
- Native Service
- Code Package
- Native Projects
- Native Call Log
- Native Task Plan Tests
- Native Windows Tools
- Native Workflows
- Native Operations
- Desktop Instance
- Native Clarifications
- Native Codex Home
- Native Execution Health
- Native Export Templates
- Native Task Gate
- Desktop Windows Schema
- Desktop Windows Schema
- Code Tsconfig
- Native Migration Activation
- Native Data Backup
- Native Task Finish
- Native Asset Agent
- Native Documents
- Native Task Plan
- Native Task Control
- Native Test Support
- Desktop Desktop Schema
- Native Blueprint
- Native Tests Code Structure Contract
- Desktop Desktop Schema
- Desktop Setup Runtime
- Desktop Validation Runtime
- Code Package
- Native Scheduler
- Desktop Tauri Conf
- Native Export Dependencies
- Native Rpc
- Native Asset Sessions
- Native Language
- Native Task Integration Tests
- Native Scheduler Worker
- Native Tools
- Native Repository
- Native Comparison
- Native Release Display
- Native Task Completion
- Native Tests Agent
- Native Rpc
- Native Task Relations
- Native Release
- Native Task Gate Asset Tests
- Desktop Asset Task Preview

## God Nodes (most connected - your core abstractions)
1. `safe_path()` - 73 edges
2. `Backend` - 59 edges
3. `Task` - 44 edges
4. `Store` - 40 edges
5. `call()` - 38 edges
6. `Project` - 34 edges
7. `safePath()` - 34 edges
8. `Tasks` - 31 edges
9. `scripts` - 31 edges
10. `Preferences` - 29 edges

## Surprising Connections (you probably didn't know these)
- `health()` --indirect_call--> `time()`  [INFERRED]
  tests/task-execution-health.test.ts → src/ui/validation/types.ts
- `FakeRpc` --inherits--> `CodexRpc`  [EXTRACTED]
  tests/recovery.test.ts → src/core/rpc.ts
- `ThreadBoundTests` --uses--> `BoundedServer`  [INFERRED]
  tests/asset_observer_bounds_test.py → resources/workflows/asset_observer_transport.py
- `StateTests` --uses--> `ObserverState`  [INFERRED]
  tests/asset_observer_bounds_test.py → resources/workflows/asset_observer_transport.py
- `TransportTests` --uses--> `ObserverState`  [INFERRED]
  tests/asset_observer_transport_test.py → resources/workflows/asset_observer_transport.py

## Import Cycles
- None detected.

## Communities (193 total, 20 thin omitted)

### Community 10 - "Shared Game Design"
Cohesion: 0.09
Nodes (43): Context, CampaignKey, PriorityKey, OverviewDraft, TaskProjectContext, OverviewChoice, add(), defaultGameBrief() (+35 more)

### Community 11 - "Electron Core Export Templates"
Cohesion: 0.12
Nodes (31): Games, DesktopTool, main(), main(), bundleFiles(), captureExportBundle(), verifyExportBundle(), boundedText() (+23 more)

### Community 13 - "Prototypes Prototype"
Cohesion: 0.11
Nodes (44): Game, GameTab, Page, SettingsTab, Task, button(), closeModal(), element() (+36 more)

### Community 15 - "Electron Core Settings"
Cohesion: 0.07
Nodes (27): Preview, StartProof, LocalApiDefaults, Preferences, Vault, Store, Api, Capability (+19 more)

### Community 16 - "Electron Core Files"
Cohesion: 0.09
Nodes (30): Files, ProjectLocks, FileOperation, Journal, Change, Snapshot, documentPath(), readDocument() (+22 more)

### Community 18 - "Electron Core Executor"
Cohesion: 0.10
Nodes (14): ActiveTask, LocalExecutor, CodexRpc, Clarification, TaskPlan, TaskExecutor, object(), validateAnswers() (+6 more)

### Community 19 - "UI Api"
Cohesion: 0.17
Nodes (7): WindowCommand, WindowState, Window, NativeApi, nativeTitlebarCommand(), windowCommands, statusNames

### Community 22 - "Shared Autonomy"
Cohesion: 0.10
Nodes (30): AskRatio, TaskDirection, TaskEvent, Document, Resource, automaticChoice(), autonomyLabel(), shouldAutomate() (+22 more)

### Community 23 - "UI Studio Preview"
Cohesion: 0.10
Nodes (31): Reference, Run, Document, IconName, Choice, StudioAsset, StudioNote, StudioPage (+23 more)

### Community 28 - "UI Notification State"
Cohesion: 0.15
Nodes (24): Page, Notification, NotificationInput, NotificationTone, App(), AssetTitlebar(), DesktopShell(), ExportGameDialog() (+16 more)

### Community 32 - "UI Feedback Draft"
Cohesion: 0.14
Nodes (22): AssetFeedback, AssetReference, AssetSubmission, DraftStorage, FeedbackDraft, MemoryStorage, archiveDraft(), clearDraft() (+14 more)

### Community 49 - "Scripts Native Release"
Cohesion: 0.14
Nodes (13): PayloadFile, Package, main(), main(), main(), inventory(), verifyNativeRelease(), main() (+5 more)

### Community 5 - "UI Preview Controls"
Cohesion: 0.10
Nodes (33): AssetAnnotation, AssetFrame, AssetPick, AssetStage, AssetTaskState, ObserverStatus, ObserverView, Point (+25 more)

### Community 50 - "Electron Core Tasks"
Cohesion: 0.27
Nodes (3): Tasks, AssetWindowState, Task

### Community 51 - "Electron Core Tool Setup"
Cohesion: 0.19
Nodes (11): ToolSetup, ToolName, ToolSetupState, ToolStatus, managedCodex(), matchesToolVersion(), assetSchema, idSchema (+3 more)

### Community 59 - "Electron Core Projects"
Cohesion: 0.19
Nodes (13): Projects, ProjectBlueprint, ProjectOverview, Asset, Feature, GameBrief, Project, State (+5 more)

### Community 6 - "UI Validation Types"
Cohesion: 0.10
Nodes (45): Category, Definition, Flow, Region, SourceReference, Step, Source, CodeReport (+37 more)

### Community 7 - "Prototypes Prototype"
Cohesion: 0.09
Nodes (57): Game, GameTab, Page, SettingsTab, Task, button(), closeModal(), element() (+49 more)

### Community 8 - "Resources Asset Observer Transport"
Cohesion: 0.05
Nodes (19): Observer, BoundedServer, ObserverState, ObserverView, ScopedServer, StateTests, ThreadBoundTests, TransportTests (+11 more)

### Community 81 - "Shared Engine Packages"
Cohesion: 0.18
Nodes (15): EnginePackage, PlanningPackage, BlueprintDraft, CallRecord, filterPackages(), planningPackages(), featureCategory(), suggestedBlueprintFeatures() (+7 more)

### Community 132 - "Scripts Build Native"
Cohesion: 0.26
Nodes (7): main(), encodeTree(), main(), writeIfChanged(), buildBrandAssets(), renderBrandIcon(), iconSizes

### Community 152 - "UI Documents View"
Cohesion: 0.50
Nodes (4): DocumentsView(), refresh(), save(), loadDrafts()

### Community 154 - "Scripts Native Media Smoke"
Cohesion: 0.67
Nodes (3): main(), call(), rpc()

### Community 155 - "Scripts Native Performance Smoke"
Cohesion: 0.67
Nodes (3): main(), task(), wait()

### Community 20 - "Prototypes Prototype"
Cohesion: 0.18
Nodes (37): button(), closeModal(), element(), escape(), generateAsset(), hydrateIcons(), icon(), modal() (+29 more)

### Community 30 - "Prototypes Prototype"
Cohesion: 0.21
Nodes (30): button(), closeModal(), element(), escape(), hydrateIcons(), icon(), modal(), newProject() (+22 more)

### Community 80 - "Scripts Package"
Cohesion: 0.12
Nodes (14): app, branded, electronPath, executable, files, groups, icons, licenses (+6 more)

### Community 87 - "Prototypes App"
Cohesion: 0.20
Nodes (11): colorToRgba(), contrastRatio(), copyText(), relativeLuminance(), renderContrast(), renderTokens(), showToast(), dialogLayer (+3 more)

### Community 94 - "Scripts Desktop Smoke"
Cohesion: 0.19
Nodes (9): tree(), verifyBlueprint(), verifyPackages(), { _electron }, env, errors, fixtureHome, output (+1 more)

### Community 0 - "Native Asset Task"
Cohesion: 0.06
Nodes (67): DynamicImage, Method, final_frame(), prune(), Arc, Mutex, Path, Result (+59 more)

### Community 1 - "Native Preferences"
Cohesion: 0.07
Nodes (61): Aes256Gcm, conversion_is_atomic_idempotent_and_keeps_originals(), LegacyVault, migrate(), prepare(), Prepared, Connection, Path (+53 more)

### Community 100 - "Desktop Business Api"
Cohesion: 0.24
Nodes (11): Api, capabilities(), port(), AppHandle, Arc, Option, Result, String (+3 more)

### Community 101 - "Native Capture"
Cohesion: 0.29
Nodes (9): Capture, Capture, dimensions(), encode_bgra(), png_conversion_preserves_channels_and_ignores_row_padding(), AtomicBool, Result, Value (+1 more)

### Community 102 - "Native Launch"
Cohesion: 0.21
Nodes (11): prepare(), Resources, BTreeMap, OsString, Path, PathBuf, Result, Store (+3 more)

### Community 103 - "Native Reveal"
Cohesion: 0.30
Nodes (11): only_registered_project_and_task_targets_are_revealed(), open(), resolve(), Path, PathBuf, Result, Store, String (+3 more)

### Community 104 - "Native Scheduler"
Cohesion: 0.29
Nodes (11): Arc, Factory, Files, Fn, Mutex, Self, Send, Store (+3 more)

### Community 105 - "Native Task Resources"
Cohesion: 0.35
Nodes (11): bytes(), decode_text(), known_encodings_are_lossless_and_unknown_bytes_remain_available(), live_changes_deletions_and_references_are_discoverable(), raw(), resources(), Result, String (+3 more)

### Community 106 - "Native Release Tests"
Cohesion: 0.44
Nodes (11): absent_flows_and_foreign_receipts_cannot_silently_reduce_release_scope(), disabling_visual_checks_changes_only_new_candidates_and_never_waives_code(), export_input(), finish_runs(), formal_export_is_bound_to_the_frozen_candidate_scope_and_artifacts(), human_visual_approval_cannot_clear_code_failures_or_missing_task_coverage(), Fixture, Release (+3 more)

### Community 107 - "Desktop Desktop Schema"
Cohesion: 0.17
Nodes (12): $ref, description, items, type, uniqueItems, description, items, type (+4 more)

### Community 108 - "Desktop Business Mcp"
Cohesion: 0.27
Nodes (9): Bridge, Arc, AtomicBool, Option, Result, String, Value, run() (+1 more)

### Community 109 - "Native Migration Import Tests"
Cohesion: 0.27
Nodes (10): Fixture, invalid_paths_or_credentials_cannot_partially_convert_entities(), lexical_paths_handle_namespaces_and_reject_escape_and_prefix_collision(), missing_codex_history_or_unknown_schema_keeps_import_pending(), prepared_import_rebases_before_journal_and_preserves_originals(), PathBuf, Result, String (+2 more)

### Community 110 - "Native Scheduler"
Cohesion: 0.38
Nodes (5): Message, Result, String, Scheduler, UnboundedSender

### Community 111 - "Native Task Brief"
Cohesion: 0.51
Nodes (10): choice(), format(), joined(), name(), prompt(), prompt_preserves_user_intent_references_stop_and_review_boundary(), Option, String (+2 more)

### Community 112 - "Native Settings"
Cohesion: 0.29
Nodes (9): read(), Default, Result, Self, Store, String, Value, save() (+1 more)

### Community 113 - "Desktop Task Control"
Cohesion: 0.25
Nodes (7): call(), AppHandle, Arc, Option, Result, String, Value

### Community 114 - "Native Executor Tools"
Cohesion: 0.20
Nodes (9): dispatch(), Arc, Context, Mutex, Option, Result, Store, String (+1 more)

### Community 115 - "Native Feedback Context"
Cohesion: 0.56
Nodes (9): safe_path(), freeze(), freeze_feedback(), freeze_repair(), Files, Path, Result, Run (+1 more)

### Community 116 - "Native Approval Tests"
Cohesion: 0.33
Nodes (9): confirm(), confirmation_is_bound_to_user_run_snapshot_and_evidence(), incomplete_or_changed_evidence_cannot_be_confirmed(), partial_approval_and_automatic_pass_never_advance_the_human_baseline(), restart_preserves_partial_evidence_and_does_not_resume_an_incomplete_run(), Fixture, Result, Run (+1 more)

### Community 117 - "Desktop Desktop Schema"
Cohesion: 0.22
Nodes (10): description, required, type, Capability, description, required, type, Capability (+2 more)

### Community 118 - "Desktop Desktop Schema"
Cohesion: 0.20
Nodes (10): type, webviews, windows, items, description, items, type, description (+2 more)

### Community 119 - "Desktop Windows Schema"
Cohesion: 0.20
Nodes (10): type, webviews, windows, items, description, items, type, description (+2 more)

### Community 12 - "Native Flow"
Cohesion: 0.07
Nodes (37): BTreeSet, Action, authored(), Config, Definition, Reference, relative(), Default (+29 more)

### Community 120 - "Desktop Business Catalog"
Cohesion: 0.24
Nodes (6): Result, String, Value, Vec, tools(), validate()

### Community 121 - "Desktop Template Runtime"
Cohesion: 0.27
Nodes (8): Active, cancel(), prepare(), Drop, Option, PathBuf, Result, Value

### Community 122 - "Native Asset Tool"
Cohesion: 0.33
Nodes (8): definition(), error_result(), image_result(), projection(), Feedback, State, Value, text_result()

### Community 123 - "Desktop Desktop Schema"
Cohesion: 0.22
Nodes (9): description, properties, required, type, CapabilityRemote, urls, description, type (+1 more)

### Community 124 - "Desktop Windows Schema"
Cohesion: 0.22
Nodes (9): description, properties, required, type, CapabilityRemote, urls, description, type (+1 more)

### Community 125 - "Desktop Asset Task Runtime"
Cohesion: 0.44
Nodes (7): call(), AppHandle, Arc, Result, Value, state(), submit()

### Community 126 - "Desktop Asset Task Windows"
Cohesion: 0.39
Nodes (8): authorize(), authorize_image(), open(), release(), AppHandle, Result, String, Value

### Community 127 - "Desktop Shell"
Cohesion: 0.25
Nodes (8): beaver_window(), AppHandle, Result, String, Value, WebviewWindow, run(), show_main()

### Community 128 - "Native Asset Withdrawal"
Cohesion: 0.43
Nodes (7): cancel(), cancel_feedback(), pause_empty_followup(), Feedback, Result, State, Store

### Community 129 - "Native Coverage"
Cohesion: 0.36
Nodes (7): refresh(), Path, Result, Store, String, Value, watch()

### Community 130 - "Desktop Business Routing"
Cohesion: 0.25
Nodes (7): call(), AppHandle, Arc, Option, Result, String, Value

### Community 131 - "Desktop Task Runtime"
Cohesion: 0.25
Nodes (7): AppHandle, Arc, Mutex, Path, Result, Store, start()

### Community 133 - "Native Rpc"
Cohesion: 0.48
Nodes (4): Reply, Rpc, Arc, Duration

### Community 134 - "Native Executor Tools Tests"
Cohesion: 0.48
Nodes (6): asset_validation_error_reaches_model_and_next_call_still_works(), capture_rpc(), captured(), Receiver, Result, Value

### Community 135 - "Native Export Receipt"
Cohesion: 0.43
Nodes (6): rechecking_keeps_provenance_even_when_bundle_is_broken(), Result, String, Value, same_path(), verification()

### Community 136 - "Native Source Encoding"
Cohesion: 0.33
Nodes (6): normalize(), Path, Result, String, Vec, sources_normalize_without_touching_binary_or_invalid_bytes()

### Community 137 - "Native Confirmation"
Cohesion: 0.38
Nodes (6): confirm(), Path, Result, Run, Store, String

### Community 138 - "Native Coordinator"
Cohesion: 0.52
Nodes (6): refresh(), refresh_feedback(), Path, Result, Store, Value

### Community 139 - "Native Evidence"
Cohesion: 0.43
Nodes (6): media_path(), Path, PathBuf, Result, Run, validate()

### Community 14 - "Native Migration Bundle"
Cohesion: 0.13
Nodes (42): Entry, command(), complete_bundle_restores_without_originals_and_keeps_database_bytes(), create(), ensure_activated(), exact_names(), fixture(), incomplete_or_overlapping_projects_never_silently_omitted() (+34 more)

### Community 140 - "Native Runner"
Cohesion: 0.29
Nodes (6): execute(), AtomicBool, Fn, Option, Path, Run

### Community 141 - "Desktop Windows Schema"
Cohesion: 0.29
Nodes (7): $ref, description, items, type, uniqueItems, items, permissions

### Community 142 - "Desktop Game Runtime"
Cohesion: 0.43
Nodes (6): call(), prepare_play(), PathBuf, Result, String, Value

### Community 143 - "Scripts Icon Review Smoke"
Cohesion: 0.29
Nodes (6): { chromium }, errors, expectedCandidates, folder, lastChoice, output

### Community 144 - "Native Rpc Handshake"
Cohesion: 0.27
Nodes (4): main(), Result, main(), Result

### Community 145 - "Desktop Workflow Runtime"
Cohesion: 0.40
Nodes (4): call(), Result, String, Value

### Community 148 - "Native Automatic Repair"
Cohesion: 0.60
Nodes (4): refresh(), Path, Result, Store

### Community 149 - "Scripts Verify Release"
Cohesion: 0.40
Nodes (4): manifest, manifestPath, root, { version }

### Community 150 - "Native Executor Contract"
Cohesion: 0.83
Nodes (3): fixture(), main(), Result

### Community 151 - "Native Rpc Contract"
Cohesion: 0.83
Nodes (3): fixture(), main(), Result

### Community 153 - "Desktop Validation Catalog"
Cohesion: 0.50
Nodes (3): Value, Vec, tools()

### Community 17 - "Native Blender Session"
Cohesion: 0.09
Nodes (34): addon(), ping(), Request, reservations_are_distinct_and_released_without_starting_blender(), Arc, AtomicBool, Drop, Mutex (+26 more)

### Community 2 - "Native Tests Fixture"
Cohesion: 0.05
Nodes (58): accept(), build(), duplicate(), receipt(), reserve_delivery(), Feedback, Option, Reference (+50 more)

### Community 21 - "Native Capture Windows"
Cohesion: 0.09
Nodes (26): BOOL, Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession, HWND, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D (+18 more)

### Community 24 - "Native Game Play"
Cohesion: 0.10
Nodes (29): main(), Path, Result, wait_for(), ChildGroup, closed_player_rejects_launch_before_touching_executable(), drain(), Job (+21 more)

### Community 25 - "Native Store"
Cohesion: 0.12
Nodes (24): events_keep_latest_five_hundred_in_chronological_order(), failed_transaction_rolls_back_all_writes(), preserves_legacy_rows_and_unknown_fields(), Connection, FnOnce, Option, Path, Result (+16 more)

### Community 26 - "Desktop Backend"
Cohesion: 0.11
Nodes (22): Backend, business_call(), AppHandle, Arc, AtomicBool, Mutex, Option, PathBuf (+14 more)

### Community 27 - "Native Journal"
Cohesion: 0.19
Nodes (25): external_edit_aborts_only_confirmed_writes(), FileOperation, fixture(), get(), Journal, Journal<'a>, now(), OperationKind (+17 more)

### Community 29 - "Native Game Export"
Cohesion: 0.17
Nodes (29): check_stage(), config(), custom_template(), custom_template_belongs_to_selected_preset(), delivery_record(), engine_path(), engine_paths_remove_only_windows_namespace_prefix(), execute() (+21 more)

### Community 3 - "Native Validation Model"
Cohesion: 0.06
Nodes (58): BytesStart, Decoder, directories(), execute(), green(), attributes(), parse_report(), Result (+50 more)

### Community 31 - "Code Package"
Cohesion: 0.06
Nodes (31): scripts, build, build:icons, build:native, build:native:ui, check, check:effective-lines, check:effective-lines:strict (+23 more)

### Community 33 - "Desktop Business Api"
Cohesion: 0.09
Nodes (22): Bytes, HeaderMap, Json, main(), Result, main(), Result, main() (+14 more)

### Community 34 - "Native Media"
Cohesion: 0.13
Nodes (26): bounded(), Input, Media, Provider, BTreeMap, Option, PathBuf, Response (+18 more)

### Community 35 - "Native Assets"
Cohesion: 0.15
Nodes (27): AssetResponse, byte_range(), capture_is_published_once_under_registered_project(), import_files(), import_rejects_linked_destination(), import_root(), imports_keep_bytes_names_and_previous_success_on_failure(), only_registered_roots_are_readable() (+19 more)

### Community 36 - "Native Tool Setup"
Cohesion: 0.16
Nodes (19): cancellation_preserves_installed_tools_and_restart_never_installs(), concurrent_setup_is_rejected_and_failure_releases_guard(), dependency_order_and_redetection_are_required_before_acceptance(), Fixture, idle(), Operations, recover(), Arc (+11 more)

### Community 37 - "Desktop Asset Runtime"
Cohesion: 0.25
Nodes (7): call(), AppHandle, Arc, Option, Result, String, Value

### Community 38 - "Native Files"
Cohesion: 0.25
Nodes (17): Metadata, cached_capture_rejects_corruption_and_detects_same_size_same_mtime_edits(), Change, failed_batch_restores_earlier_writes(), file_hash(), Files, linked(), list_files() (+9 more)

### Community 39 - "Native Executor"
Cohesion: 0.19
Nodes (18): Future, Control, Execution, interrupt(), Outcome, Arc, Command, Context (+10 more)

### Community 4 - "Native Policy"
Cohesion: 0.06
Nodes (52): OsString, Result, Vec, run(), is_source(), measure(), Result, Baseline (+44 more)

### Community 40 - "Native Task Actions"
Cohesion: 0.14
Nodes (24): create(), embedded_packages_and_updates_preserve_customization_and_adoption_boundary(), Manifest, Option, Path, Result, Store, String (+16 more)

### Community 41 - "Native Task Create"
Cohesion: 0.15
Nodes (24): asset_state_failure_rolls_back_task_event_and_related_receipt(), fixture(), queued_asset_state_survives_interruption_without_starting_blender(), PathBuf, Result, Store, TempDir, create() (+16 more)

### Community 42 - "Native Service"
Cohesion: 0.13
Nodes (21): Arc, AtomicBool, BTreeMap, Drop, Fn, JoinHandle, Mutex, Option (+13 more)

### Community 43 - "Code Package"
Cohesion: 0.08
Nodes (25): devDependencies, electron, esbuild, pe-library, prettier, resedit, @resvg/resvg-js, tsx (+17 more)

### Community 44 - "Native Projects"
Cohesion: 0.17
Nodes (23): automatic(), effective(), recommendation(), Result, Store, Value, set(), thresholds_inheritance_and_overrides() (+15 more)

### Community 45 - "Native Call Log"
Cohesion: 0.22
Nodes (21): Activity, begin(), correlates_pages_and_recovers_without_persisting_secret_payloads(), finish(), link(), now(), query(), recover() (+13 more)

### Community 46 - "Native Task Plan Tests"
Cohesion: 0.15
Nodes (22): manual_approval_blocks_dependents_and_restart_preserves_plan(), missing_plan_and_conflicts_never_create_or_approve_children(), plan(), plan_creates_real_children_once_and_uses_latest_approved_files(), rejects_stale_duplicate_and_incomplete_plans(), Files, Result, Store (+14 more)

### Community 47 - "Native Windows Tools"
Cohesion: 0.15
Nodes (21): collect(), executable(), file(), hint_path(), launcher(), natural_cmp(), registered(), registered_paths_launchers_and_templates() (+13 more)

### Community 48 - "Native Workflows"
Cohesion: 0.21
Nodes (23): call(), camera_schema(), configured_engine(), engine_path(), Input, install(), list(), NprOptions (+15 more)

### Community 52 - "Native Operations"
Cohesion: 0.23
Nodes (21): create(), Path, Result, Run, Store, Value, selection(), enqueue() (+13 more)

### Community 53 - "Desktop Instance"
Cohesion: 0.18
Nodes (17): activate(), Activation, Instance, name(), Arc, AtomicBool, Drop, File (+9 more)

### Community 54 - "Native Clarifications"
Cohesion: 0.29
Nodes (21): answer(), answer_with_auto(), Choice, fully_automatic_questions_continue_running(), length(), mixed_policy_preserves_manual_choices_and_records_automatic_sources(), park(), parked_question_survives_restart_and_answer_is_not_replayed() (+13 more)

### Community 55 - "Native Codex Home"
Cohesion: 0.19
Nodes (19): atomic_write(), copy_skills(), HomeRequest, isolates_config_and_keeps_secrets_out_of_files(), latency_overrides(), prepare(), PreparedHome, BTreeMap (+11 more)

### Community 56 - "Native Execution Health"
Cohesion: 0.16
Nodes (12): Health, IdleLimits, is_tool(), now(), retries_and_usage_do_not_hide_a_stall_but_tools_and_output_do(), Default, Duration, HashSet (+4 more)

### Community 57 - "Native Export Templates"
Cohesion: 0.26
Nodes (20): archive_install_is_allowlisted_versioned_and_non_overwriting(), checksum(), directory(), download(), download_from(), download_hash_bounds_and_exclusive_destination(), exact_versions_and_unique_official_checksums(), install_archive() (+12 more)

### Community 58 - "Native Task Gate"
Cohesion: 0.20
Nodes (20): candidate(), changes(), execute(), prepare(), ready(), required(), Arc, AtomicBool (+12 more)

### Community 60 - "Desktop Windows Schema"
Cohesion: 0.10
Nodes (19): anyOf, definitions, Identifier, Number, PermissionEntry, Target, Value, description (+11 more)

### Community 61 - "Desktop Windows Schema"
Cohesion: 0.10
Nodes (20): properties, default, description, type, description, type, default, description (+12 more)

### Community 62 - "Code Tsconfig"
Cohesion: 0.10
Nodes (19): compilerOptions, esModuleInterop, jsx, lib, module, moduleResolution, noUncheckedIndexedAccess, skipLibCheck (+11 more)

### Community 63 - "Native Migration Activation"
Cohesion: 0.22
Nodes (18): Manifest, activate(), activation_requires_matching_archive_and_exclusive_ownership(), changed_project_path_blocks_activation_before_tool_execution(), check_task(), check_tree(), fixture(), read_json() (+10 more)

### Community 64 - "Native Data Backup"
Cohesion: 0.32
Nodes (18): backup_rejects_directory_links_without_touching_target(), copy_entries(), create(), Entry, hold_database(), inventory(), Manifest, new_destination() (+10 more)

### Community 65 - "Native Task Finish"
Cohesion: 0.32
Nodes (18): awaiting(), completed_output_merges_durably_and_cannot_be_replayed(), finish(), finish_task(), human_conflict_blocks_entire_batch(), matching_external_changes_are_not_owned_by_task_rollback(), retry_merge(), retry_rejects_unfinished_review_and_shutdown_states() (+10 more)

### Community 66 - "Native Asset Agent"
Cohesion: 0.20
Nodes (10): Context, Arc, Mutex, Option, PathBuf, Result, State, Store (+2 more)

### Community 67 - "Native Documents"
Cohesion: 0.25
Nodes (17): assets(), document_path(), documents_reject_hidden_paths_invalid_utf8_and_oversize(), edits_are_revision_checked_journaled_and_keep_baseline(), project_path(), read_document(), read_limited(), Option (+9 more)

### Community 68 - "Native Task Plan"
Cohesion: 0.25
Nodes (17): approval(), eligible(), expand(), Plan, prepare(), reconcile(), Files, Path (+9 more)

### Community 69 - "Native Task Control"
Cohesion: 0.14
Nodes (17): Arc, AtomicBool, Files, Fn, Mutex, Option, PathBuf, Receiver (+9 more)

### Community 70 - "Native Test Support"
Cohesion: 0.22
Nodes (10): Fixture, Files, Flow, Option, PathBuf, Result, Run, Self (+2 more)

### Community 71 - "Desktop Desktop Schema"
Cohesion: 0.11
Nodes (17): anyOf, definitions, Number, PermissionEntry, Target, Value, description, anyOf (+9 more)

### Community 72 - "Native Blueprint"
Cohesion: 0.38
Nodes (16): catalog(), catalog_choice(), choice(), defaults_and_custom_theme_normalize_without_extra_fields(), number(), object(), project_plan(), ratings_online_capacity_and_priority_constraints_hold() (+8 more)

### Community 73 - "Native Tests Code Structure Contract"
Cohesion: 0.25
Nodes (14): code_structure_does_not_bless_same_length_legacy_edits(), code_structure_mcp_preflight_is_read_only_and_needs_no_media_credentials(), code_structure_merge_retry_uses_captured_output_not_a_rewritten_workspace(), code_structure_rejects_the_entire_merge_and_allows_split_recovery(), Fixture, Files, PathBuf, Result (+6 more)

### Community 74 - "Desktop Desktop Schema"
Cohesion: 0.12
Nodes (17): properties, Identifier, default, description, type, description, oneOf, type (+9 more)

### Community 75 - "Desktop Setup Runtime"
Cohesion: 0.24
Nodes (10): Adapter, prepare(), recover(), AppHandle, AtomicBool, Result, Store, String (+2 more)

### Community 76 - "Desktop Validation Runtime"
Cohesion: 0.18
Nodes (16): call(), ffmpeg(), is_query(), media(), query(), AppHandle, Arc, Mutex (+8 more)

### Community 77 - "Code Package"
Cohesion: 0.12
Nodes (16): dependencies, react, react-dom, smol-toml, three, zod, description, main (+8 more)

### Community 78 - "Native Scheduler"
Cohesion: 0.17
Nodes (15): Id, Active, cancel(), claim(), dropping_last_handle_interrupts_unstarted_queue(), interrupt_queued(), Launch, AtomicBool (+7 more)

### Community 79 - "Desktop Tauri Conf"
Cohesion: 0.12
Nodes (15): app, security, windows, withGlobalTauri, build, frontendDist, bundle, active (+7 more)

### Community 82 - "Native Export Dependencies"
Cohesion: 0.27
Nodes (14): Architecture, collect(), imports(), lookup(), only_imported_libraries_are_copied_recursively_without_overwrite(), pe(), rejects_mismatched_dependency_architecture(), rejects_path_imports_and_invalid_binaries() (+6 more)

### Community 83 - "Native Rpc"
Cohesion: 0.15
Nodes (13): AsyncMutex, AtomicU64, ChildStdin, Inner, read_messages(), AsyncRead, AtomicBool, Child (+5 more)

### Community 84 - "Native Asset Sessions"
Cohesion: 0.27
Nodes (9): Arc, HashMap, Mutex, Option, Result, Session, Store, String (+1 more)

### Community 85 - "Native Language"
Cohesion: 0.18
Nodes (8): Language, Option, Self, count(), opening(), Quoted, Option, Vec

### Community 86 - "Native Task Integration Tests"
Cohesion: 0.37
Nodes (14): automatic_repairs_stop_on_repetition_budget_or_missing_engine(), code_delivery_does_not_wait_for_visuals_and_preserves_manual_approval(), concurrent_project_changes_require_fresh_code_before_recorded_output_merges(), delivery(), finish(), green_code_receipt_never_overwrites_a_human_conflict(), late_user_instructions_survive_validation_and_prevent_premature_merge(), parent_auto_completion_requires_integrated_code_and_accepted_children() (+6 more)

### Community 88 - "Native Scheduler Worker"
Cohesion: 0.15
Nodes (13): execute(), Arc, AtomicBool, Factory, Files, Fn, Mutex, Receiver (+5 more)

### Community 89 - "Native Tools"
Cohesion: 0.27
Nodes (13): candidate(), configured_path_is_checked_without_running_it(), detect(), detect_one(), find(), native_codex(), AtomicBool, Option (+5 more)

### Community 9 - "Native Repository"
Cohesion: 0.09
Nodes (48): capture(), complete_bundle_rejects_missing_modified_extra_and_undeclared_entry(), Manifest, Record, Path, Result, String, Value (+40 more)

### Community 90 - "Native Comparison"
Cohesion: 0.31
Nodes (13): baseline_run(), compare(), compare_with(), event_steps(), Baseline, Option, Path, Result (+5 more)

### Community 91 - "Native Release Display"
Cohesion: 0.25
Nodes (12): list(), prepare(), Prepared, Option, Path, Release, Result, Run (+4 more)

### Community 92 - "Native Task Completion"
Cohesion: 0.41
Nodes (13): child(), delivered(), freeze_repair(), integrated(), repair(), resume(), Files, Option (+5 more)

### Community 93 - "Native Tests Agent"
Cohesion: 0.33
Nodes (13): a_turn_change_during_checkpoint_cannot_commit_stale_stages(), checkpoint_server(), context(), missing_identity_and_other_tasks_cannot_read_the_asset_tool(), params(), poll_carries_actual_png_and_acknowledgement_waits_for_transport_delivery(), Fixture, JoinHandle (+5 more)

### Community 95 - "Native Rpc"
Cohesion: 0.32
Nodes (7): Event, Command, Receiver, Result, Self, String, Value

### Community 96 - "Native Task Relations"
Cohesion: 0.29
Nodes (12): create(), create_seeded(), prefix(), relations_freeze_current_project_and_preserve_parent_and_context(), Feedback, Option, Path, Result (+4 more)

### Community 97 - "Native Release"
Cohesion: 0.41
Nodes (12): authorize(), engine_version(), inspect(), display(), Path, Release, Result, Store (+4 more)

### Community 98 - "Native Task Gate Asset Tests"
Cohesion: 0.40
Nodes (12): assets_and_feedback_must_finish_before_preparing_code_validation(), delivery(), feedback(), finish(), late_asset_feedback_invalidates_old_code_and_recaptures_resumed_edits(), pass(), ready(), Fixture (+4 more)

### Community 99 - "Desktop Asset Task Preview"
Cohesion: 0.37
Nodes (12): client(), freeze(), image(), pick(), reference(), Arc, Option, Result (+4 more)

## Knowledge Gaps
- **304 isolated node(s):** `Context`, `DesktopTool`, `Game`, `GameTab`, `Page` (+299 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **20 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `safe_path()` connect `Native Feedback Context` to `Native Asset Task`, `Native Coverage`, `Native Validation Model`, `Native Policy`, `Desktop Task Runtime`, `Native Source Encoding`, `Native Repository`, `Native Evidence`, `Native Migration Bundle`, `Native Blender Session`, `Native Journal`, `Native Game Export`, `Native Media`, `Native Assets`, `Native Files`, `Native Task Actions`, `Native Task Create`, `Native Projects`, `Native Workflows`, `Native Operations`, `Native Codex Home`, `Native Migration Activation`, `Native Data Backup`, `Native Task Finish`, `Native Documents`, `Native Task Plan`, `Native Export Dependencies`, `Native Comparison`, `Native Task Relations`, `Native Reveal`, `Native Task Resources`?**
  _High betweenness centrality (0.169) - this node is a cross-community bridge._
- **Why does `Backend` connect `Desktop Backend` to `Desktop Business Routing`, `Desktop Game Runtime`, `Desktop Workflow Runtime`, `Native Game Play`, `Native Assets`, `Native Tool Setup`, `Desktop Asset Runtime`, `Native Service`, `Native Windows Tools`, `Desktop Instance`, `Desktop Setup Runtime`, `Desktop Validation Runtime`, `Desktop Asset Task Preview`, `Desktop Business Api`, `Native Capture`, `Native Scheduler`, `Desktop Task Control`, `Desktop Template Runtime`, `Desktop Asset Task Runtime`, `Desktop Asset Task Windows`?**
  _High betweenness centrality (0.091) - this node is a cross-community bridge._
- **Why does `Scheduler` connect `Native Scheduler` to `Desktop Task Runtime`, `Native Scheduler`, `Desktop Validation Runtime`, `Native Scheduler`, `Native Asset Sessions`, `Desktop Backend`?**
  _High betweenness centrality (0.079) - this node is a cross-community bridge._
- **Are the 64 inferred relationships involving `safe_path()` (e.g. with `capture()` and `import_files()`) actually correct?**
  _`safe_path()` has 64 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Context`, `DesktopTool`, `Game` to the rest of the system?**
  _304 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Shared Game Design` be split into smaller, more focused modules?**
  _Cohesion score 0.09413008989952407 - nodes in this community are weakly interconnected._
- **Should `Electron Core Export Templates` be split into smaller, more focused modules?**
  _Cohesion score 0.1196808510638298 - nodes in this community are weakly interconnected._