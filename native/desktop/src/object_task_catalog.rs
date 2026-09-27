use serde_json::{json, Value};
#[path = "object_feedback_relocation_catalog.rs"]
mod relocation;

pub(crate) fn tools() -> Vec<Value> {
    let revision = json!({"type":"integer","minimum":0,"maximum":i64::MAX});
    let mut anchor = id();
    anchor["type"] = json!(["string", "null"]);
    let definitions = [
        (
            "objectTask.declarePlanningComplete",
            "Owner acknowledgement that the current responsibility subtree is fully planned. Requires a fresh snapshot scope hash and plan revision, no pending planning notes or empty active parent branches. Stores an immutable idempotent receipt. Does not accept tasks or authorize execution; changes to scope definitions or cancellations invalidate the current declaration.",
            schema(
                json!({"projectId":id(),"taskId":id(),"requestId":id(),"expectedPlanRevision":revision,"expectedScopeHash":{"type":"string","pattern":"^[a-f0-9]{64}$"},"reason":{"type":"string","minLength":1,"maxLength":2000}}),
                &["projectId", "taskId", "requestId", "expectedPlanRevision", "expectedScopeHash", "reason"],
            ),
            false,
        ),
        (
            "objectTask.saveDraft",
            "Save a project-local object task draft with draft and plan revision checks. Does not create executable tasks.",
            schema(
                json!({"projectId":id(),"draftId":id(),"expectedRevision":revision,"expectedPlanRevision":revision,"plan":plan()}),
                &["projectId", "draftId", "expectedRevision", "expectedPlanRevision", "plan"],
            ),
            false,
        ),
        (
            "objectTask.getDraft",
            "Restore a saved draft from an open project. Returns null when it does not exist.",
            query(Some("draftId")),
            true,
        ),
        (
            "objectTask.unlockDraft",
            "Start an empty editable draft after commit without changing committed tasks. Checks draft and current plan revisions. Retry the identical requestId and payload to recover the original result.",
            schema(
                json!({"projectId":id(),"requestId":id(),"draftId":id(),"expectedRevision":revision,"expectedPlanRevision":revision}),
                &["projectId", "requestId", "draftId", "expectedRevision", "expectedPlanRevision"],
            ),
            false,
        ),
        (
            "objectTask.commit",
            "Atomically persist draft objects, task hierarchy, runs and an idempotent receipt. Reuse requestId when retrying. Committed medium tasks can be explicitly enqueued for execution.",
            schema(
                json!({"projectId":id(),"requestId":id(),"draftId":id(),"expectedDraftRevision":revision,"expectedPlanRevision":revision}),
                &["projectId", "requestId", "draftId", "expectedDraftRevision", "expectedPlanRevision"],
            ),
            false,
        ),
        (
            "objectTask.cancelPlanned",
            "Withdraw a planned task and its unstarted responsibility descendants, closing owned medium runs. Rejects started work; preserves history and independent tasks even when they share objects or dependencies. Check task and plan revisions; reuse requestId when retrying.",
            schema(
                json!({"projectId":id(),"taskId":id(),"requestId":id(),"expectedTaskRevision":revision,"expectedPlanRevision":revision}),
                &["projectId", "taskId", "requestId", "expectedTaskRevision", "expectedPlanRevision"],
            ),
            false,
        ),
        (
            "objectTask.revisePlanned",
            "Revise the title, goal, acceptance, required/optional classification, pending planning notes and dependencies of an unstarted task. Requires a reason and task/plan revisions. Preserves identity and stores before/after history, owner adoption and affected task IDs atomically. Reuse the full request with requestId on retry.",
            schema(
                json!({"projectId":id(),"taskId":id(),"requestId":id(),"expectedTaskRevision":revision,"expectedPlanRevision":revision,"definition":definition(),"reason":{"type":"string","minLength":1,"maxLength":2000}}),
                &["projectId", "taskId", "requestId", "expectedTaskRevision", "expectedPlanRevision", "definition", "reason"],
            ),
            false,
        ),
        (
            "objectTask.setCoarsePaused",
            "Pause or unpause new dispatch for all current and future medium children of a coarse task. Preserves child controls, queue order and active leases. Check task and control revisions; retry with the identical request. Unpause does not authorize recovery.",
            schema(
                json!({"projectId":id(),"taskId":id(),"requestId":id(),"expectedTaskRevision":revision,"expectedControlRevision":{"type":"integer","minimum":0,"maximum":9_007_199_254_740_990_u64},"paused":{"type":"boolean"}}),
                &["projectId", "taskId", "requestId", "expectedTaskRevision", "expectedControlRevision", "paused"],
            ),
            false,
        ),
        (
            "objectTask.setPaused",
            "Pause or unpause future dispatch for one medium task. Check task and dispatch-control revisions; reuse the identical requestId and payload on retry. Paused queue heads block later work for the same object. An already dispatched preparation or execution continues; use interrupt to stop it. Unpause only restores queue eligibility; failed or orphaned runs still require recovery and awaiting-acceptance work stays gated.",
            schema(
                json!({"projectId":id(),"taskId":id(),"objectId":id(),"runId":id(),"requestId":id(),"expectedTaskRevision":revision,"expectedControlRevision":{"type":"integer","minimum":0,"maximum":9_007_199_254_740_990_u64},"paused":{"type":"boolean"}}),
                &["projectId", "taskId", "objectId", "runId", "requestId", "expectedTaskRevision", "expectedControlRevision", "paused"],
            ),
            false,
        ),
        (
            "objectTask.revisions",
            "Read immutable definition revision history ordered by task revision, including after cancellation. Requires an existing task in an open project.",
            query(Some("taskId")),
            true,
        ),
        (
            "objectTask.snapshot",
            "Read a consistent plan revision, object task hierarchy, runs and dispatch controls from an open project. A medium with no saved dispatch control defaults to unpaused at control revision zero.",
            query(None),
            true,
        ),
        (
            "objectTask.get",
            "Read an object task by its project-local ID. Returns null when absent; never resolves a legacy task.",
            query(Some("taskId")),
            true,
        ),
        (
            "objectTask.getRun",
            "Read an object run from an open project. Its baseline is resolved when the scheduler prepares its workspace.",
            query(Some("runId")),
            true,
        ),
        (
            "objectTask.attempts",
            "Read compact execution records for a project-local run. Availability distinguishes live execution, frozen output, and orphaned running records that require recovery. Does not restart work.",
            query(Some("runId")),
            true,
        ),
        (
            "objectTask.attemptFile",
            "Read an immutable checkpoint member up to 1 MiB as UTF-8 text, PNG/JPEG/GIF/WebP image, or WAV/MP3/Ogg/FLAC audio. Never reads the live workspace; oversized files are not integrity-verified by this preview.",
            schema(json!({"projectId":id(),"runId":id(),"attemptId":id(),"checkpoint":{"type":"string","enum":["input","output"]},"path":{"type":"string","minLength":1},"sha256":{"type":"string","pattern":"^[a-f0-9]{64}$"}}), &["projectId", "runId", "attemptId", "checkpoint", "path", "sha256"]),
            true,
        ),
        (
            "objectTask.attemptTrace",
            "Read the first 256 persisted operation metadata events for an attempt. Excludes commands, provider text and stderr; truncation is explicit. Does not restart work.",
            schema(json!({"projectId":id(),"runId":id(),"attemptId":id()}), &["projectId", "runId", "attemptId"]),
            true,
        ),
        (
            "objectTask.checkAttempt",
            "Persist point-in-time integrity and changed-code structure evidence for a frozen attempt. Retry the identical request to recover its report; a new request rechecks. Does not accept, publish, release ownership or launch work.",
            schema(json!({"projectId":id(),"requestId":id(),"target":attempt_target()}), &["projectId", "requestId", "target"]),
            false,
        ),
        (
            "objectTask.prepareCandidateReview",
            "Freeze a whole-output review for the current successful final fine using its exact passing check report. Shows claim-baseline diff, fixed references, ownership and publication blockers. Exact retry returns historical evidence. Never accepts, materializes, releases ownership or starts work.",
            schema(json!({"projectId":id(),"requestId":id(),"target":attempt_target(),"checkRequestId":id()}), &["projectId", "requestId", "target", "checkRequestId"]),
            false,
        ),
        (
            "objectTask.candidateReviews",
            "Read immutable whole-output candidate reviews for an attempt, including retained history. Reports describe their capture time, not current publication eligibility.",
            query(Some("attemptId")),
            true,
        ),
        (
            "objectTask.attemptChecks",
            "Read immutable historical technical reports for a frozen attempt. Does not recheck current files.",
            query(Some("attemptId")),
            true,
        ),
        (
            "objectTask.publicationPreview",
            "Review the current accepted-head diff, complete owned files and rework feedback for a frozen final candidate. Validates current source records. Read-only; does not accept or start execution.",
            schema(json!({"projectId":id(),"target":attempt_target(),"reviewRequestId":id()}), &["projectId", "target", "reviewRequestId"]),
            true,
        ),
        (
            "objectTask.publishCandidate",
            "Explicit owner acceptance of the reviewed final candidate. Requires the exact preview digest, owned-file confirmation, replacement consent when required and a disposition for every feedback. Freezes content, journals recoverable file writes, then atomically accepts the version and releases the queue. Retry the identical request after a pending or uncertain response; query publications after reopen. A pending result retains ownership.",
            schema(json!({
                "projectId":id(),"requestId":id(),"target":attempt_target(),"reviewRequestId":id(),
                "previewDigest":{"type":"string","pattern":"^[0-9a-f]{64}$"},
                "acceptanceNote":{"type":"string","minLength":1,"maxLength":4000},
                "confirmFiles":{"type":"boolean","enum":[true]},"confirmReplacement":{"type":"boolean"},
                "feedback":{"type":"array","maxItems":500,"items":schema(json!({
                    "requestId":id(),"resolution":{"type":"string","enum":["resolved","waived","deferred"]},
                    "note":{"type":"string","minLength":1,"maxLength":4000}
                }), &["requestId", "resolution", "note"])}
            }), &["projectId", "requestId", "target", "reviewRequestId", "previewDigest", "acceptanceNote", "confirmFiles", "confirmReplacement", "feedback"]),
            false,
        ),
        (
            "objectTask.publications",
            "Read durable publication operations for one medium, including completed history. Reopening or querying never publishes or starts work. Pending operations retain the exact request needed for retry or abort.",
            query(Some("taskId")),
            true,
        ),
        (
            "objectTask.deferCandidateFeedback",
            "Save immutable feedback for a current publishable candidate. Publication requires explicit deferred disposition and atomically creates planned follow-up work pinned to the new version. Does not enqueue or execute. Retry the identical request after uncertainty.",
            schema(json!({
                "projectId":id(),"requestId":id(),
                "review":schema(json!({"projectId":id(),"target":attempt_target(),"reviewRequestId":id()}), &["projectId", "target", "reviewRequestId"]),
                "feedback":{"type":"string","minLength":1,"maxLength":4000},
                "later":schema(json!({"title":{"type":"string","minLength":1,"maxLength":300},"acceptance":{"type":"string","minLength":1,"maxLength":10000}}), &["title", "acceptance"]),
                "image":rework_image(),
                "relocation":relocation::schema(),
                "previewFrame":schema(json!({"runId":id(),"frameId":id()}), &["runId", "frameId"])
            }), &["projectId", "requestId", "review", "feedback", "later"]),
            false,
        ),
        (
            "objectTask.createPublicationFollowup",
            "Explicitly create planned medium and fine work from published feedback, pinned to that publication version. Atomically retains source provenance and an idempotent receipt. Does not enqueue, execute, accept or change project files. Retry the identical request after an uncertain response.",
            schema(json!({
                "projectId":id(),"requestId":id(),"publicationRequestId":id(),"versionId":id(),
                "title":{"type":"string","minLength":1,"maxLength":300},
                "feedback":{"type":"string","minLength":1,"maxLength":16000},
                "acceptance":{"type":"string","minLength":1,"maxLength":10000},
                "previewFrame":schema(json!({"runId":id(),"frameId":id()}), &["runId", "frameId"])
            }), &["projectId", "requestId", "publicationRequestId", "versionId", "title", "feedback", "acceptance"]),
            false,
        ),
        (
            "objectTask.publicationFrames",
            "Read immutable numbered Godot frames belonging to this exact published object version for explicit follow-up feedback. Never starts a preview or task.",
            query(Some("publicationRequestId")),
            true,
        ),
        (
            "objectTask.attemptFrames",
            "Read the latest eight numbered frames from this attempt output, or one exact archived reference for historical replay. Never launches an engine or changes tasks.",
            schema(json!({"projectId":id(),"attemptId":id(),
                "previewFrame":schema(json!({"runId":id(),"frameId":id()}), &["runId", "frameId"])
            }), &["projectId", "attemptId"]),
            true,
        ),
        (
            "objectTask.publicationFollowups",
            "Read durable follow-up creation receipts for a publication. Never creates or starts work.",
            query(Some("publicationRequestId")),
            true,
        ),
        (
            "objectTask.abortPublication",
            "Explicitly abort an unfinished publication identified by its original requestId. Reverses only journaled writes still matching the candidate; retains external changes and frozen work. Does not cancel the task or release ownership. Repeat the same request after interruption; published versions cannot be aborted.",
            schema(json!({"projectId":id(),"requestId":id(),"confirmAbort":{"type":"boolean","enum":[true]}}), &["projectId", "requestId", "confirmAbort"]),
            false,
        ),
        (
            "objectTask.recovery",
            "Read the medium run's latest verification and disposition journal, including retained history after cancellation. Does not scan files, launch work or release ownership. `reportMatchesRecords` compares durable records only. `canDispose` requires current stop/content/workspace evidence with no pause or unresolved disposition. Retained history cannot grant writer rights. A null result means workspace preparation has not been recorded.",
            query(Some("taskId")),
            true,
        ),
        (
            "objectTask.verifyRecovery",
            "Verify retained run records, frozen content and workspace without starting execution or releasing ownership. Reuse the complete request and requestId after an uncertain response or pending operation. Results are evidence at one generation, not permission to continue or cancel. Missing live execution does not prove writers stopped.",
            schema(json!({"projectId":id(),"requestId":id(),"target":recovery_target(false)}), &["projectId", "requestId", "target"]),
            false,
        ),
        (
            "objectTask.resumeRecovery",
            "Explicitly retry a stopped failed or interrupted fine from its verified checkpoint. Rechecks workspace and ownership, creates one new attempt, and retains history. Reuse the complete request after an uncertain response. Completed replay never launches work; reopening never resumes automatically.",
            schema(json!({"projectId":id(),"requestId":id(),"target":recovery_target(true),"verificationRequestId":id()}), &["projectId", "requestId", "target", "verificationRequestId"]),
            false,
        ),
        (
            "objectTask.advanceAttempt",
            "Explicitly accept the checked successful fine and launch its immediate planned successor. Requires an owner acceptance note, exact next fine revision and passing technical report. Rechecks frozen content, workspace and ownership; retains accepted stage history and object ownership. Exact replay never launches again. Does not accept the final fine or publish an object.",
            schema(json!({"projectId":id(),"requestId":id(),"target":recovery_target(true),"verificationRequestId":id(),"advance":schema(json!({"attemptId":id(),"checkRequestId":id(),"nextFineTaskId":id(),"nextFineRevision":{"type":"integer","minimum":0,"maximum":9_007_199_254_740_990_u64},"acceptanceNote":{"type":"string","minLength":1,"maxLength":4000}}), &["attemptId", "checkRequestId", "nextFineTaskId", "nextFineRevision", "acceptanceNote"])}), &["projectId", "requestId", "target", "verificationRequestId", "advance"]),
            false,
        ),
        (
            "objectTask.reworkCandidate",
            "Rework the reviewed final fine with explicit owner feedback and optional frozen output PNG regions. PNG must decode, match the source hash and dimensions, and fit 1 MiB and 16 megapixels. Regions use normalized top-left coordinates, numbered by array order. Requires current candidate review and recovery verification. Preserves previous attempts, accepted stages and ownership. Exact replay never launches again; does not accept or publish the candidate.",
            schema(json!({"projectId":id(),"requestId":id(),"target":recovery_target(true),"verificationRequestId":id(),"rework":schema(json!({"reviewRequestId":id(),"attemptId":id(),"fineTaskId":id(),"feedback":{"type":"string","minLength":1,"maxLength":4000},"image":rework_image(),"relocation":relocation::schema(),"previewFrame":schema(json!({"runId":id(),"frameId":id()}), &["runId", "frameId"])}), &["reviewRequestId", "attemptId", "fineTaskId", "feedback"])}), &["projectId", "requestId", "target", "verificationRequestId", "rework"]),
            false,
        ),
        (
            "objectTask.disposeRecovery",
            "Cancel the exact verified medium and its unstarted fine tasks; `cancelAndKeep` retains the workspace, while `cancelAndRemoveWorkspace` removes only the verified task workspace after rechecking ownership. Rechecks stop and integrity evidence before atomically releasing queue ownership. Executed fine and attempt history remain unchanged. Blocked results retain ownership and require a new verification; pending or uncertain results require the identical full request and requestId. Does not accept a version or resume execution.",
            schema(json!({"projectId":id(),"requestId":id(),"target":recovery_target(true),"verificationRequestId":id(),"choice":{"type":"string","enum":["cancelAndKeep","cancelAndRemoveWorkspace"]}}), &["projectId", "requestId", "target", "verificationRequestId", "choice"]),
            false,
        ),
        (
            "objectTask.interrupt",
            "Interrupt only the exact live attempt identified by its full target and task revision. Waits for process cleanup and output capture; retains workspace and object ownership. Retry the identical request and requestId to recover its frozen receipt. A late request returns the actual frozen outcome. Orphaned running records require recovery.",
            schema(
                json!({"projectId":id(),"requestId":id(),"target":attempt_target(),"expectedTaskRevision":revision}),
                &["projectId", "requestId", "target", "expectedTaskRevision"],
            ),
            false,
        ),
        (
            "objectTask.enqueue",
            "Add planned medium tasks with their owned runs to the persistent object queue and wake the scheduler. Eligible work prepares its workspace and executes its first fine task. Repeating the same request is idempotent.",
            schema(
                json!({"projectId":id(),"taskIds":{"type":"array","items":id(),"minItems":1,"maxItems":500,"uniqueItems":true}}),
                &["projectId", "taskIds"],
            ),
            false,
        ),
        (
            "objectTask.queue",
            "Read the persisted object queue in position order. This is read-only and does not claim work.",
            query(None),
            true,
        ),
        (
            "objectTask.claim",
            "Claim the next eligible medium task for one owner. Preserve each object's queue head until all dependencies are accepted; other objects can proceed. Claimed, awaiting-acceptance and failed runs retain exclusive ownership. Each claim carries a generation token.",
            schema(
                json!({"projectId":id(),"owner":{"type":"string","minLength":1,"maxLength":128}}),
                &["projectId", "owner"],
            ),
            false,
        ),
        (
            "objectTask.reorder",
            "Move a queued task between adjacent queued anchors from queueView. Requires its reviewed version; concurrent changes require review again. Reuse the identical request after an uncertain response. Preserves active ownership and blocked same-object heads.",
            schema(
                json!({"projectId":id(),"requestId":id(),"taskId":id(),"expectedVersion":{"type":"string","pattern":"^[0-9a-f]{64}$"},"previousTaskId":anchor,"nextTaskId":anchor}),
                &["projectId", "requestId", "taskId", "expectedVersion", "previousTaskId", "nextTaskId"],
            ),
            false,
        ),
        (
            "objectTask.queueView",
            "Read a transactional queue review with optimistic version and blocker explanations. Does not claim or start work.",
            query(None),
            true,
        ),
        (
            "objectTask.finish",
            "Record a claimed run's result only when owner, claim token and generation match. Success awaits acceptance; failure awaits recovery. Both retain object ownership. Duplicate or stale callbacks are rejected.",
            schema(
                json!({"projectId":id(),"taskId":id(),"owner":{"type":"string","minLength":1,"maxLength":128},"claimToken":{"type":"string","minLength":1,"maxLength":128},"generation":revision,"success":{"type":"boolean"}}),
                &["projectId", "taskId", "owner", "claimToken", "generation", "success"],
            ),
            false,
        ),
        (
            "objectTask.cancelClaim",
            "Cancel metadata for an owned claim and its responsibility descendants using owner, claim token and generation. Supports claimed, awaiting-acceptance and failed runs; advances the plan revision and releases queue ownership. Does not stop processes or dispose files. Stale callbacks are rejected.",
            schema(
                json!({"projectId":id(),"taskId":id(),"owner":{"type":"string","minLength":1,"maxLength":128},"claimToken":{"type":"string","minLength":1,"maxLength":128},"generation":revision}),
                &["projectId", "taskId", "owner", "claimToken", "generation"],
            ),
            false,
        ),
    ];
    definitions
        .into_iter()
        .map(|(name, description, input_schema, read)| {
            json!({
                "name":name,"description":description,"inputSchema":input_schema,
                "annotations":{"readOnlyHint":read,"destructiveHint":false,"openWorldHint":false}
            })
        })
        .collect()
}

fn attempt_target() -> Value {
    let text = json!({"type":"string","minLength":1,"maxLength":128});
    let rpc_id = json!({"type":["string","null"],"minLength":1,"maxLength":128});
    schema(
        json!({
            "taskId":id(),"fineTaskId":id(),"objectId":id(),"runId":id(),"attemptId":id(),
            "owner":text,"claimToken":text,
            "generation":{"type":"integer","minimum":1,"maximum":i64::MAX},
            "threadId":rpc_id,"turnId":rpc_id
        }),
        &[
            "taskId",
            "fineTaskId",
            "objectId",
            "runId",
            "attemptId",
            "owner",
            "claimToken",
            "generation",
            "threadId",
            "turnId",
        ],
    )
}

fn recovery_target(disposition: bool) -> Value {
    let revision = json!({"type":"integer","minimum":0,"maximum":9_007_199_254_740_991_u64});
    let text = json!({"type":"string","minLength":1,"maxLength":128});
    schema(
        json!({
            "taskId":id(),"objectId":id(),"runId":id(),"owner":text,"claimToken":text,
            "writerGeneration":{"type":"integer","minimum":1,"maximum":9_007_199_254_740_991_u64},
            "taskRevision":revision,"runRevision":revision,"objectRevision":revision,"controlRevision":revision,
            "recoveryGeneration":{"type":"integer","minimum":if disposition { 1 } else { 0 },"maximum":if disposition { 9_007_199_254_740_991_u64 } else { 9_007_199_254_740_990_u64 }}
        }),
        &[
            "taskId",
            "objectId",
            "runId",
            "owner",
            "claimToken",
            "writerGeneration",
            "taskRevision",
            "runRevision",
            "objectRevision",
            "controlRevision",
            "recoveryGeneration",
        ],
    )
}

fn id() -> Value {
    json!({"type":"string","minLength":1,"maxLength":128,"pattern":"^[A-Za-z0-9_.:-]+$"})
}

fn schema(properties: Value, required: &[&str]) -> Value {
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}

fn query(field: Option<&str>) -> Value {
    let mut properties = json!({"projectId":id()});
    let mut required = vec!["projectId"];
    if let Some(field) = field {
        properties[field] = id();
        required.push(field);
    }
    schema(properties, &required)
}

fn definition() -> Value {
    schema(
        json!({
            "title":{"type":"string","minLength":1,"maxLength":300},
            "prompt":{"type":"string","minLength":1,"maxLength":20000},
            "acceptance":{"type":"string","maxLength":10000},
            "requirement":requirement(),"pendingPlanning":pending_planning(),
            "dependsOn":{"type":"array","items":id(),"maxItems":500,"uniqueItems":true}
        }),
        &["title", "prompt", "acceptance", "dependsOn"],
    )
}

fn plan() -> Value {
    let object = schema(
        json!({
            "id":id(),"name":{"type":"string","minLength":1,"maxLength":256},
            "category":{"type":"string","minLength":1,"maxLength":128,"default":"其他"}
        }),
        &["id", "name"],
    );
    let mut optional_id = id();
    optional_id["type"] = json!(["string", "null"]);
    let task = schema(
        json!({
            "id":id(),"granularity":{"type":"string","enum":["coarse","medium","fine"]},
            "position":{"type":"integer","minimum":0,"maximum":1_000_000_000},
            "title":{"type":"string","minLength":1,"maxLength":300},
            "prompt":{"type":"string","minLength":1,"maxLength":20000},
            "acceptance":{"type":"string","maxLength":10000},
            "requirement":requirement(),"pendingPlanning":pending_planning(),
            "objectId":optional_id,"parentTaskId":optional_id,"stageId":optional_id,"baseline":baseline(),
            "dependsOn":{"type":"array","items":id(),"maxItems":500,"uniqueItems":true}
        }),
        &["id", "granularity", "title", "prompt", "acceptance"],
    );
    let assumption = schema(
        json!({
            "id":id(),"statement":{"type":"string","minLength":1,"maxLength":2000},
            "basis":{"type":"string","minLength":1,"maxLength":4000},
            "source":{"type":"string","enum":["user","codex","automatic"]},
            "sourceDetail":{"type":["string","null"],"maxLength":1000}
        }),
        &["id", "statement", "basis"],
    );
    schema(
        json!({
            "objects":{"type":"array","items":object,"maxItems":128},
            "tasks":{"type":"array","items":task,"maxItems":500},
            "assumptions":{"type":"array","items":assumption,"maxItems":500}
        }),
        &[],
    )
}

fn baseline() -> Value {
    json!({"oneOf":[
        schema(json!({"basePolicy":{"const":"latestAccepted"}}), &["basePolicy"]),
        schema(json!({"basePolicy":{"const":"pinnedVersion"},"selectedVersionId":id()}), &["basePolicy","selectedVersionId"]),
        schema(json!({"basePolicy":{"const":"empty"}}), &["basePolicy"])
    ]})
}

fn requirement() -> Value {
    json!({"type":"string","enum":["required","optional"],"default":"required",
        "description":"Required by default. Optional work still obeys explicit dependencies and acceptance gates."})
}

fn pending_planning() -> Value {
    json!({"type":"string","maxLength":4000,"default":"",
        "description":"Unexpanded requirements for coarse or medium tasks only; fine tasks must leave this empty. Core enforces a 4000 UTF-8 byte limit. Empty does not declare planning or goal completion."})
}

fn rework_image() -> Value {
    let coordinate = json!({"type":"number","minimum":0,"maximum":1});
    let extent = json!({"type":"number","exclusiveMinimum":0,"maximum":1});
    let region = schema(
        json!({"x":coordinate,"y":coordinate,"width":extent,"height":extent,
        "prompt":{"type":"string","maxLength":1000}}),
        &["x", "y", "width", "height", "prompt"],
    );
    let dimension = json!({"type":"integer","minimum":1,"maximum":16384});
    schema(
        json!({"path":{"type":"string","maxLength":1024,"pattern":"\\.[pP][nN][gG]$"},
        "sha256":{"type":"string","pattern":"^[a-f0-9]{64}$"},"width":dimension,"height":dimension,
        "regions":{"type":"array","minItems":1,"maxItems":8,"items":region}}),
        &["path", "sha256", "width", "height", "regions"],
    )
}
