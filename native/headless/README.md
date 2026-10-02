# Beaver headless host

This binary exposes Core project creation and opt-in external-agent execution over JSONL stdin/stdout.
It is not an HTTP service, object scheduler, replacement AI provider or fixture.
Only project-local storage is accepted. Use a dedicated data directory; the same
`.beaver-native.lock` excludes Desktop or another headless host using that directory.
Project `.beaver` locks also prevent two hosts from opening the same project.

## Build

From the repository root, generate the ordinary native resources with
`npm run build:native:ui`, then `cargo build -p beaver-headless`.
This does not build Tauri. The executable also supports `--media-mcp`, delegating
directly to Core's stdio media/workflow MCP server.

## Transport

Run `beaver-headless --data-dir /absolute/path/to/host-data`.
Each line is an object with `id` (number or string up to 200 bytes), `method`, and
optional object `input`. A line is limited to 1 MiB including its newline.
Each response has the same `id` and either `result` or `error.message`.
Requests are admitted serially; task execution remains Core Scheduler's job.
Clients must send one request and wait for its response before sending the next.
The transport holds at most one queued frame plus one overflow-detection frame,
each capped at 1 MiB. Additional input while the queued frame is still unadmitted
terminates the transport with a pipeline error on stderr and a nonzero exit;
those unadmitted frames are not executed. Partial extra input can also trigger
this fail-closed limit. This bound prevents input backlogs from hiding EOF behind a blocked
stdout writer. Keep stdin open while waiting for responses.
EOF is a disconnect: the active accepted call drains, but
unadmitted buffered requests are not guaranteed to run. Use `shutdown` for an
explicit close after all desired responses have arrived.
Unknown methods, fields, wrong top-level types, malformed JSON and oversized lines
are rejected without executing them. Input is not a shell language.

Example requests (send the returned project ID in subsequent requests):

```json
{"id":1,"method":"project.create","input":{"parent":"/absolute/existing/parent","name":"BlankTest","template":"blank"}}
{"id":2,"method":"settings.get"}
{"id":3,"method":"workflow.list","input":{"id":"RETURNED_PROJECT_ID"}}
{"id":4,"method":"task.create","input":{"projectId":"RETURNED_PROJECT_ID","prompt":"Create a simple scene","decompose":false}}
{"id":5,"method":"state"}
{"id":6,"method":"shutdown"}
```

Methods:

- `project.create`: Core project parameters; optional `npr: {godot: absolutePath}`
- `project.npr.install`: `id`, `godot`; preserves failed/partial installation evidence
- `workflow.list`: project `id`
- `workflow.run`: project `id`, `workflow`, `action`; Core optional definition/camera/grayscale
- `task.create`: Core legacy parameters; optional `executionMode: "external-agent"`
  opts into the external runner. Omission preserves Responses execution. External
  `assetTask: true` and object identities are rejected
- `task.events`, `task.interrupt`: task `id`
- `task.continue`: `id`, `text`; optional `freshContext`
- `task.answer`: `id`, `questionId`, `answers`; optional `automatic`
- `external.pending`: no input; currently claimed external run contexts
- `external.context`: `taskId`; only ready after Scheduler has claimed and registered a run
- `external.history`: `taskId`; read durable run history, including after restart
- `external.receipt`: `taskId`, `runId`, `requestId`; inspect durable admission/result
  even after the original host exits (this grants no authority to resume a run)
- `external.submitPlan`, `external.tool`, `external.finish`: `taskId`, `runId`,
  `revision` (unsigned integer), `requestId`, object `arguments`
- `state`: project-local projects/tasks plus redacted host settings
- `logs.query`: optional `taskId`, `projectId`, `method`, `after`, `limit`
- `settings.get`; `settings.save`: complete `settings`, optional empty `keys: {}`
- `shutdown`: revoke external runs, drain the admitted call and owned Scheduler,
  clean up managed process trees, then release project runtime locks

Read `settings.get`, edit the complete returned object, then submit it as
`settings.save.input.settings`. Non-secret provider configuration needs an existing
Responses-compatible endpoint and its model, for example `mode: "local"` with
`local.code.baseUrl` and `local.code.model` (and equivalent fields for capabilities
actually used). A non-local endpoint must use HTTPS. Configure real executable
paths in `tools`; settings do not install them.

`settings.save` accepts no credential entries, including empty named slots. There
is no plaintext fallback for Linux's unsupported SystemVault, and this Linux host
has no verified secure credential-input path. An `.env` file is not a supported
workaround. Existing encrypted credentials remain under Core's vault policy; the
settings API returns only `hasKey` flags, never decrypted keys. For the default Responses mode, a configured real
Responses provider and real compatible tools are required; absent configuration
causes an actual queued task to fail through Core's scheduler, without an invented
thread, Blender session or project merge. This host alone does not prove end-to-end
character creation.

## External-agent execution

Create a task with `executionMode: "external-agent"`, then poll `external.pending`
or request `external.context` for its task ID. The host owns its volatile identity;
no caller JSON can supply an owner, Codex `threadId`, or `turnId`. Run IDs and
revision numbers come from the real Scheduler claim and durable run record. This
mode needs no Responses credentials or Codex executable. Godot and Blender paths
still come from ordinary tool settings and installed project workflow settings.

`external.tool.arguments` contains `{ "tool": "file.read", "arguments": { ... } }`.
The context includes exact schemas for `file.read`, `file.write`, `blender.start`
(`blender.python` alias), `blender.poll`, and `blender.cancel`. File writes and
Blender outputs require an explicit expected SHA-256, or null to create a new
file. Blender starts return a managed job ID; poll it before finishing. The job
receipt records the executable, arguments, PID, script hash, logs and output hashes.
The transport-wide 1 MiB line limit includes all JSON escaping and applies even
when a Core tool permits a larger raw argument.

Use the current `runId` and `revision` for every mutation, with a new `requestId`.
An identical request can return its saved receipt; conflicting parameters and
stale revisions are rejected. Unknown outcomes must be inspected, never replayed
automatically. Plan submission uses Core plan expansion and child mode inheritance.
A completed finish requests ordinary Core validation and merge; it cannot certify
a validation pass or bypass it. Interruption/restart revokes old runs.

Blender Python is trusted code execution with the host user's OS permissions.
Workspace path checks and isolated job HOME are not an OS sandbox. Beaver manages
job directories, owned process trees, declared staged outputs and their receipts;
do not grant untrusted agents access to this local stdio API. No public server,
persistent token, OAuth grant or plaintext-secret transport is added.

## Shutdown and recovery

Startup replays each project's Journal, recovers tasks and validation runs once,
and changes prior queued/running tasks to interrupted. It never silently resumes
model consumption. Task answers and continuation are explicit API calls. EOF,
SIGINT, SIGTERM and output failure revoke external runs first, allowing admitted
tools to cancel and persist receipts while their admission permit remains held.
Ordinary admitted mutations still finish. The host then stops the owned Scheduler,
drains framework jobs, releases external-run runtime leases and closes projects.
An audit-log completion failure is reported on stderr without changing an already
committed operation's response into a failure. Sending `shutdown` is the preferred deterministic close; queued
bytes after shutdown or a received termination signal are not admitted.
If termination or EOF arrives while stdout is blocked, response delivery is abandoned
so shutdown can finish. The final response may therefore be partial or absent;
that does not roll back a committed mutation. Reopen and inspect persisted state
before retrying a mutation whose response was lost.

Cleanup uncertainty is a durable recovery barrier. A run whose process/job cleanup
was not confirmed, including an abrupt host crash, cannot be resumed automatically
or replaced on the same task workspace. Its task, run and evidence remain available;
this version does not offer a bypass or automatic recovery-unlock operation.
`external.jobEvidence` accepts only `taskId`, `runId` and `requestId` for a recorded
Blender start. It reads the matching historical job report through host-resolved
storage. No caller path or old PID is accepted as authority, and even a historical
success report never clears the recovery barrier.

A receipt with `status: succeeded` means the API request was handled. For example,
a Blender start may still be running, and a workflow result can have `ok: false`.
Inspect the actual job/validation result; request success is not character acceptance.
