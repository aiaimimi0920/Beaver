# Current pinned NPR integration

The native/headless integration now targets `NPRCharacterFrame` commit
`a08b47a6cc229b6978afda26d74d13af690a67ba`, with model contract **1.1.0** and
plugin metadata **1.3.0**. These are separate versions. The older RoleNPR/R15
records below are historical evidence, not acceptance of this new package.

- The distribution includes 344 byte-pinned framework, authoring, and checker
  files. Silver Wolf sample assets and their sample-only launchers are excluded.
  The sample editor menu is not enabled; runtime installation still configures
  shader globals and Forward+.
- The compiled provenance is authoritative. Install, discovery, inspection,
  validation and preview reject missing, modified or linked framework files;
  editing the project's copy of the provenance cannot authorize altered code.
- Existing `npr_characters` projects retain their selected engine but require an
  explicit migration before the new workflow can run. Installation never silently
  replaces user changes or installs two addons with duplicate global classes.
- Native planning can select `general` or `npr-character` per step. New explicit
  choices freeze the source/version/contract identity on the child and are checked
  again before execution. A general task is not forced to use NPR. The Electron
  prototype retains its older schema and rejects the native-only field.
- `validate` invokes the official `check_model.gd` with an explicit definition.
  Its raw report remains separate from Beaver's envelope. A structural pass is
  not a rendered-image or artistic-quality pass. Preview preserves fixed-camera
  and grayscale options and executes only after the official check passes.
- Definition/model hashes and framework hashes have explicit coverage fields.
  External character textures, material profiles and feature-data dependencies
  are not claimed to have complete hash coverage by those fields.

The opt-in headless host and its external-agent execution contract are documented
in [the headless README](../native/headless/README.md). External-agent mode uses
real Beaver task/run/tool records and does not impersonate a Codex session or
export an assistant's API credentials. Blender Python is trusted code with host
OS access; path and staging rules are not an OS sandbox. Final completion still
requires the normal validation and merge/approval path.

## Verification boundary

The focused CI workflow checks the host, run protocol, tools and package adapter.
These tests do not establish character-creation or MiDot Forward+ acceptance.
That acceptance requires a new project, actual agent-selected task steps,
Beaver-owned Blender output, the pinned framework's official report, and actual
MiDot-rendered views of the newly authored character. Stock Godot, a sample
character, a fixture response, or a Blender beauty render cannot replace it.

---

# NPR package integration and call analysis

Implementation contract, 2026-09-10. R15 positive character acceptance passed;
see [the acceptance record](R15-ACCEPTANCE.md).

The native Beaver host remains the user-facing application. Codex chooses and
uses standard workflows and may participate in every stage. No additional AI
scheduler is introduced. The example green texture and module A are out of scope.

This slice implements project-creation NPR enablement, a project-specific custom
Godot binding, bundled RoleNPR plugin and authoring contract, task-scoped workflow
discovery/execution through MCP, and persistent correlated business/tool logs.
The existing snapshot/merge, clarification, approval and asset interfaces remain
the project lifecycle. AI decisions and executable workflow results return there.

## Package and project binding

RoleNPR source is copied from `../RoleNPR/addons/npr_characters` only. Its example
models, textures and `.godot` cache are excluded. Run `npx tsx scripts/vendor-npr.ts`
when intentionally refreshing the upstream copy. The current copy contains 98
files (380,504 bytes), with per-file SHA256 in `provenance.json`. Native builds
embed the addon, documentation, contract and provenance in the EXE. They do not
bundle the Godot editor or Blender.

In native project creation, enable the NPR checkbox and supply a custom engine
directory or editor path. The feature panel also supports installing into an
existing idle project and retrying a failed installation. The equivalent API is:

```json
{
  "method": "project.create",
  "input": {
    "parent": "C:\\Games",
    "name": "NPR project",
    "template": "blank",
    "npr": { "godot": "C:\\CustomGodot" }
  }
}
```

`project.npr.install` accepts `{ "id": "PROJECT_ID", "godot": "C:\\CustomGodot" }`.
Installation copies the addon, runs its actual settings installer, enables the
editor plugin and Forward+, probes required engine interfaces, then writes
`beaver.runtime.json`. The manifest binds subsequent task, play and workflow
operations to that project engine. It does not change global user tool settings.
Installing the package does not automatically submit a creative AI task.

Projects record `installing`, `ready` or `failed`. Failure preserves the project
and diagnostic log; identical partial files can be reused on retry. Conflicting
existing addon files are rejected without overwriting them. A successful runtime
manifest currently records installation state; it does not continuously attest
the integrity of editable project files. Engine migration for an already-ready
project is not exposed by this installation operation.

## Standard production workflow

`workflow.list { "id": "PROJECT_ID" }` discovers enabled project workflows.
`workflow.run` takes `id`, `workflow: "npr-character"`, and `action`:

- `inspect` returns the shipped authoring guide, asset contract and engine binding.
- `validate` additionally requires a project-relative `.tres` `definition`. It
  imports the project, runs the real definition validation and checks that the NPR
  actor initialized without validation errors.
- `preview` takes the same definition and renders front, side and back PNGs using
  the bound custom engine, also reporting initialization and validation results.

Reports and engine logs are retained under `artifacts/npr/<runId>`. Callers must
check the returned `ok` field and reported errors, not only the HTTP envelope.
Successful initialization and three preview views do not prove subjective art
quality or every lighting/camera requirement in the RoleNPR authoring contract.

Task-local Codex receives `beaver-workflows` and existing Godot/Blender skills,
plus `beaver_workflow_list` and `beaver_workflow_run` through the existing media
MCP server. The task workspace supplies project scope, so these two tools omit
the public API's `id`. Codex chooses the workflow, reads its contract, creates the
actual model through Blender MCP, and decides how to correct validation failures.
The editable `.blend`, exported model, required textures, definition and usable
scene belong to the ordinary task snapshot/merge and approval lifecycle.

Public install/run operations reject concurrent active creation tasks. During
creation, Codex uses the task-scoped MCP to avoid mutating the merged project.
Task interruption owns and terminates its workflow process tree. Public workflow
calls currently run until completion or application shutdown; no independent
generic workflow-cancel API is claimed. API disconnect alone does not cancel them.

Blender MCP requires an installed Blender editor and addon. NPR execution tasks
now reserve a distinct localhost port and prepare a Beaver-owned Blender GUI
session before launching Codex. Planning tasks do not start Blender. The bootstrap
loads the installed addon in an empty factory scene with isolated configuration,
script and temporary directories. Even addon auto-start uses the assigned port;
the client receives matching `BLENDER_HOST` and `BLENDER_PORT` settings. Readiness
requires both the owned process identity and an actual addon ping response.

Preparation runs outside the database lock and respects task cancellation. The
owned session closes after execution completes, fails, pauses for input or is
interrupted. Startup and verified process exit are logged as
`blender.session.start` / `blender.session.stop`; startup evidence also appears in
task events. Continuations receive a fresh empty session and must reopen their
saved `.blend` through MCP. An unrelated open Blender session is not reused or
closed. Custom export-template selection remains separate from character
installation, validation and preview.

## Call analysis

All accepted UI/API/business-MCP calls, Codex turns and relevant Codex tool calls
have persisted diagnostic spans. Query `logs.query` by project/task/method; inspect
project logs from the feature panel. See [business API logging](BUSINESS-API.md#persistent-call-logs)
for pagination, redaction, retention and recovery semantics. Workflow reports hold
the generated artifact paths; input/output summaries avoid storing raw assets in
the call table.

## Engine compatibility evidence

R14 recorded missing geometry interfaces in the old `69903a894` build. The user
then rebuilt the same selected directory, `C:\Users\Public\nas_home\godot\export`.
The new R15 preflight through Beaver API reports
`4.8.dev.custom_build.db1af1e99`, passes both required geometry-interface checks,
installs the package and records `npr.status = ready` in a new blank project.
This resolves the old engine dependency failure without switching directories.
The subsequent R15 character round also passed actual Blender MCP production,
four dependent automatically approved child tasks, NPR validation and preview,
and final merged-project preview. The operator used Beaver APIs only. Codex
connects to the user's specified `http://127.0.0.1:8317/v1` service; earlier
403/405 failures came from the old endpoint. Credentials remain in the application
vault and are excluded from evidence. A follow-up reused RoleNPR's native lab UI,
exported a Windows executable through Beaver, verified the bundle and exercised
the actual exported UI. See [native lab export acceptance](NPR-LAB-EXPORT-ACCEPTANCE.md)
for evidence and the non-blocking review-artifact merge conflict. The export
manifest's `runtimeVerified` field remains false under its existing contract.

## Acceptance boundary

Completion requires focused native/TypeScript checks, a new packaged EXE, and a
fresh blank project created through `start-fresh-native-test.ps1`. Acceptance input:
`创建一个npr 3d 白发可爱动漫女孩模型，戴着草帽。`. The operator calls Beaver APIs only. Installation,
Codex workflow discovery, actual Blender MCP calls, generated assets, NPR
validation and visible preview need recorded evidence. Minor issues are retained
and ordinary recovery continues. Prior test rounds and releases are preserved.
