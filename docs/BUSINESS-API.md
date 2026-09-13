# Beaver business API and MCP

## Status

The native Windows host now provides an opt-in local business API and a stdio
MCP bridge. Both use the same `business_call` service as the desktop Tauri command:
the same database, scheduler, project files, revision checks, recovery gates and
desktop change notifications. There is no second task executor or shadow store.

The implementation exposes 51 business operations, including task autonomy,
NPR package installation, standard workflows and call-log queries. This is a
local-owner integration preview, not the final multi-user or remote-access API.
The Electron host does not expose this API yet. R7 binaries predate this change.

## Start the native host

Set `BEAVER_API_TOKEN` to a randomly generated secret containing 32..256 ASCII
letters, digits, `_` or `-`, then launch the new native `Beaver.exe`.
Set `BEAVER_API_PORT` if port 4319 is unsuitable. The API binds only to
`127.0.0.1`; absence of the token keeps the listener disabled. Invalid configuration
or a port collision fails startup instead of silently exposing a different service.
Do not include the token in URLs, source control, logs or screenshots.

`BEAVER_DATA_DIR` still selects the host's data directory. Use a new isolated
directory for tests. The API operates on whatever data the running host owns.
Starting an MCP client does not open a second database or migrate user data.

Every request requires `Authorization: Bearer <your-token>`. Browser `Origin`
headers are rejected and no CORS access is enabled. The credential grants full
local-owner business access, including executing tools, importing credentials,
capturing windows and writing projects. Only give it to trusted clients.
This is not project-scoped authorization or a sandbox.

## HTTP contract, version 1

`GET /v1/capabilities` returns the API version, 51 tool descriptions and JSON input
schemas, project design/blueprint catalogs and execution constraints. Schemas for
design, blueprint, overview and direction are generated from the existing shared
TypeScript schemas during the native UI build. Domain refinements such as valid
catalog IDs and cross-field relationships are still enforced by the core.

`POST /v1/call` takes an object with `method` and optional `input` (defaults to `{}`).

```json
{
  "method": "project.create",
  "input": { "parent": "C:\\Games", "name": "My Game", "template": "blank" }
}
```

Successful calls return HTTP 200:

```json
{ "apiVersion": "1", "ok": true, "result": {} }
```

`result` is the real operation result; it may be a project/task, array, text, or
null. It is not always an object. Failed calls return an error envelope:

```json
{
  "apiVersion": "1",
  "ok": false,
  "error": { "code": "INVALID_INPUT", "message": "Missing field: id" }
}
```

Errors currently distinguish `UNAUTHORIZED` (401), `INVALID_JSON`,
`INVALID_REQUEST`, `INVALID_INPUT`, `METHOD_NOT_FOUND` (400), `BUSINESS_ERROR`
(409), `BUSY` (429), `SHUTTING_DOWN` (503), and `INTERNAL_ERROR` (500).
Malformed HTTP, unsupported routes and requests over the 2 MiB body limit can
receive the HTTP framework's own error response; clients must check status and
content type before decoding. Core errors retain their existing human-readable
messages; fine-grained domain error codes remain future work.

Up to 16 external calls run concurrently. Excess calls are rejected before
execution. If the client disconnects, an accepted business call continues;
desktop exit cancels owned jobs and waits for outstanding API calls before
closing the store. Mutations have no automatic retry or idempotency guarantee.
After a transport failure, inspect state before repeating a mutation.

## Business workflow

1. Discover capabilities and catalogs; read `state` for projects, tasks, redacted
   settings and feature packages.
2. `project.create` or `project.import` returns the project ID.
3. `task.create` returns a persisted task. General code tasks default to
   `decompose: true`: the AI clarifies the goal and submits executable child tasks.
   Set `decompose: false` for a single task; `autoAccept: false` selects manual
   child approval. `task.approval` updates the policy and `task.accept` approves a
   completed child. See [executable plan lifecycle](TASK-EXECUTION-PLAN.md).
   Completion of this API request means
   the task was created, not that the game has been completed.
4. Poll `state` and `task.events` for progress. Use `task.continue`,
   `task.interrupt`, or `task.answer` as appropriate. Clarification question IDs
   and answer keys come from the task's current clarification data.
5. Inspect `task.resources`, `task.resourceText`, `assets`, and documents; use
   `task.accept` or `task.rollback` according to the real task state.
6. `game.play`, `game.presets`, `game.export`, and `game.verifyExport` use the same
   installed Godot tools and export verification as the desktop.

Task completion means changes were merged and await acceptance. Acceptance records
the user's decision. Neither status implies a verified export. Project `delivery`
records the most recent export-snapshot check (`verified` or `failed`, timestamp,
path or failure message); it is also included on tasks returned by `state` for UI
display. Subsequent project edits require another export. Bundle checks keep
`runtimeVerified: false`; they do not establish playability or game quality.

Native exports import the frozen snapshot before exporting it. The selected Godot
editor must support `--import`; older editors fail with an actionable tool-setting
message instead of attempting an unreliable first-frame import. Import and export
logs are retained, and nonzero exit codes or engine errors fail verification.
`game.play` also imports the merged project before launch, because task merging
does not copy `.godot` caches. Its import log is retained under the native data
directory as `play-import-*.log`. A successful play call confirms startup only;
visual/runtime checks remain separate from bundle verification.

`task.resourceText` strictly decodes UTF-8 and BOM-marked UTF-16, without lossy
replacement. `task.resourceBytes` returns `{path, bytes, base64}` for a raw download
under the same task-resource path checks and 500 KiB limit. Changed text source
files with a recognized BOM are normalized to UTF-8 without BOM before merge;
unrecognized encodings fail safely without overwriting the original bytes.

The conversation refreshes running-task events every 1.5 seconds. A terminal
transient connection failure may resume once in the same AI thread, with a visible
recovery event. Authentication, permission, quota, context and ambiguous startup
failures are not automatically retried. The original deadline remains in force.

`document.read` returns the revision required for `document.save`. Use a null
revision only when creating a new document; an outdated revision cannot overwrite
newer content. Task/project IDs and paths retain existing core validation.

`asset.import` requires `paths`, a list of 1..100 absolute local file paths.
`game.importTemplates` requires an explicit archive `path`. External calls never
fall back to human file pickers. The desktop may continue to use its normal
pickers, then enter the same import logic. `chooseDirectory`, `chooseTool`, and
window minimize/maximize are UI-only and are not public business operations.
Reveal, play and capture operations intentionally affect the host desktop.

Game export and tool/template preparation currently keep their HTTP call open
until completion. They can run concurrently with cancellation calls. Durable
operation IDs, replayable subscriptions and resumable generic jobs are not yet
implemented. Task progress is already persisted through the existing task system.

## MCP configuration

Configure a stdio MCP client to launch the new native executable with
`--business-mcp`, passing the same token and port through the environment:

```json
{
  "mcpServers": {
    "beaver": {
      "command": "C:\\Beaver\\Beaver.exe",
      "args": ["--business-mcp"],
      "env": {
        "BEAVER_API_TOKEN": "REPLACE_WITH_YOUR_RANDOM_SECRET",
        "BEAVER_API_PORT": "4319"
      }
    }
  }
}
```

Start the desktop host first. Initialization verifies the live API connection.
The bridge implements `initialize`, `ping`, `tools/list`, and `tools/call`; tools
and schemas are discovered from the running host. It supports protocol versions
2024-11-05, 2025-03-26, 2025-06-18 and 2025-11-25, selecting 2025-11-25 for an
unsupported proposed version. Standard output contains only JSON-RPC messages.
Tool results include text and structured content; business failures set
`isError: true`. No HTTP MCP transport or remote OAuth support is claimed.

The separate `--media-mcp` interface provides four media tools and two task-scoped
workflow tools. NPR workflows use the project's bound custom engine. See
[NPR integration](NPR-INTEGRATION.md) for installation and production contracts.
The business bridge deliberately does not start another store or scheduler.
The protocol follows the official [MCP lifecycle](https://modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle)
and [tool contract](https://modelcontextprotocol.io/specification/2025-11-25/server/tools).
The local HTTP listener uses [Axum](https://docs.rs/axum/0.8.9/axum/).

## Persistent call logs

`logs.query` reads diagnostic records from the native host's existing SQLite
database. Accepted business calls share the same instrumentation for UI, API and
business MCP. Sources distinguish `ui`, `api`, `mcp`, `codex`, `codex-tool`, and
`workflow` (program-managed production preparation and cleanup).
Codex turn spans and tool items are linked to their task and project; successful
creation calls acquire the newly created IDs before completion is recorded.

```json
{
  "method": "logs.query",
  "input": { "projectId": "PROJECT_ID", "after": 0, "limit": 100 }
}
```

Optional exact filters are `taskId`, `projectId`, and `method`. `limit` is 1..500
(default 100). `records` contains call IDs, insertion sequence, source, method,
correlation IDs, timestamps, status, duration and input/output summaries.
`nextAfter` is the last returned sequence. A record initially returned as
`running` is updated in place: querying only after its sequence will not return
its later completion. Refresh the earlier page to observe those updates. This
cursor pages calls; it is not a completion-event subscription.

Summaries store serialized byte count, SHA256, top-level field names, array size
and a small allowlist of workflow/status fields. Raw argument values, generated
assets and credentials are not copied into the call table. Failed calls may add
a bounded error string with persisted credentials, the host API token and Bearer
values masked. Existing `task.events` remains the redacted narrative history.
Token usage notifications are also retained in task events when Codex emits them;
they do not establish billing cost.

The history retains the latest 50,000 sequence positions plus older active calls.
After host restart, unfinished records become `interrupted` with unknown duration
(`null`). This is local diagnostics, without tamper-proof audit guarantees.
HTTP authentication, body parsing, admission and schema failures occurring before
the shared business dispatcher are outside this table. A caller can read logs in
the native project's feature panel or through the same API/MCP operation.

## Verification and remaining work

`npm run verify:native-business -- <path-to-Beaver.exe>` starts an isolated actual
native host and real stdio MCP process. It checks authentication, discovery,
noninteractive imports, project creation, persisted document edits, revision
conflicts, path protection, shared state, and MCP mutation/error behavior.
The script stops only its own process trees. It does not contact a live model.

The current preview does not claim all 51 operations have been individually
exercised through both transports. Full live creative workflows, exit under
concurrent long-running requests, scoped grants, tamper-proof audit records,
durable request deduplication, richer output schemas, event subscriptions and a
human-facing integration settings page remain acceptance items. Migration and
backup remain separate offline CLI operations because they require ownership and
lifecycle guarantees incompatible with an active host store.
