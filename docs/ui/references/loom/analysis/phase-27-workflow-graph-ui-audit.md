# Phase 27 Workflow Graph UI Audit

## Scope

Phase 27 restores the main remaining old ArtLoom visual workflow-editor
behavior in Loom desktop without reintroducing the old ReactFlow dependency.

Restored capabilities:

- visible workflow `Graph view` inside Loom desktop Workflow Studio
- selectable workflow node cards derived from the current workflow YAML
- visual edge/dependency summary from `needs`
- `Node properties` editing for node id, `uses`, `needs`, and `with` fields
- `Add node`, `Delete node`, and `Apply node changes`
- graph edits serialized back into workflow YAML through a dedicated helper
- parity contract coverage for the new visual graph UI and graph model helpers

Visible product names remain:

- `loom.exe`
- `loom-daemon.exe`
- `loom-desktop.exe`

The old ArtLoom source remains read-only:
`Z:\project\project\ArtNexus\ArtLoom`.

## Old source evidence

Reviewed old ArtLoom visual workflow editor sources:

- `Z:\project\project\ArtNexus\ArtLoom\src\features\workflow-editor\index.tsx`
- `Z:\project\project\ArtNexus\ArtLoom\src\features\workflow-editor\components\Canvas.tsx`
- `Z:\project\project\ArtNexus\ArtLoom\src\features\workflow-editor\components\Sidebar.tsx`
- `Z:\project\project\ArtNexus\ArtLoom\src\features\workflow-editor\components\PropertiesPanel.tsx`
- `Z:\project\project\ArtNexus\ArtLoom\src\features\workflow-editor\components\RunSummaryStrip.tsx`
- `Z:\project\project\ArtNexus\ArtLoom\src\features\workflow-editor\components\nodes\ArtNode.tsx`
- `Z:\project\project\ArtNexus\ArtLoom\src\features\workflow-editor\types\graph.ts`
- `Z:\project\project\ArtNexus\ArtLoom\src\features\workflow-manager\hooks\useWorkflows.ts`
- `Z:\project\project\ArtNexus\ArtLoom\src\features\workflow-manager\components\WorkflowList.tsx`

Old editor behavior centered on:

```text
ReactFlowProvider
ReactFlow
Canvas
Sidebar
PropertiesPanel
RunSummaryStrip
MiniMap
Controls
Background
runtimeInvoke('load_workflow_data')
runtimeInvoke('save_workflow_data')
runtimeInvoke('instantiate_workflow')
runtimeInvoke('get_user_arts')
```

The product-critical behavior was not the exact ReactFlow library. It was the
visible loop that lets the user inspect a workflow graph, select nodes, edit
properties, create/delete nodes, and persist the result.

## Loom state before Phase 27

Before this phase, Phase 23 had restored a practical Workflow Studio:

- editable workflow YAML
- cURL Smart Import
- response-template inference
- workflow interface inference
- `Wrap workflow as Loom tool`
- daemon workflow save/load/delete integration from later phases

Missing relative to old ArtLoom:

- no visual graph surface
- no node card selection model
- no node properties form
- no graph-level add/delete node actions
- no serializer that applies visual graph edits back into YAML

Current desktop dependencies were intentionally small:

```json
"@tauri-apps/api"
"react"
"react-dom"
```

Adding `@xyflow/react` would have expanded the release surface and required a
larger UI port. Phase 27 therefore restores behavior through a Loom-native
lightweight graph surface while keeping the existing glass UI baseline.

## Phase 27 implementation

### Contract-first RED

Updated:

```text
scripts/tests/test-loom-artloom-parity-contract.ps1
```

New contract assertions require:

```text
Graph view
Node properties
Add node
Delete node
Apply node changes
workflowGraph
serializeWorkflowGraphLite
updateWorkflowGraphNode
addWorkflowGraphNode
deleteWorkflowGraphNode
```

The contract was run before implementation and failed as expected:

```text
Loom desktop must restore an ArtLoom-style visual workflow graph surface. Missing=[Graph view]
```

### Graph model helpers

Updated:

```text
Loom/apps/desktop/src/services/workflowStudio.ts
```

Added:

- `WorkflowGraphNodePatch`
- `serializeWorkflowGraphLite`
- `updateWorkflowGraphNode`
- `addWorkflowGraphNode`
- `deleteWorkflowGraphNode`

Behavior:

- parses existing workflow YAML through the existing `parseWorkflowYamlLite`
- serializes the edited graph back to simple workflow YAML
- keeps node ids unique on add/rename
- rewrites downstream `needs` references when a node is renamed
- removes deleted node references from downstream `needs`

### Desktop Workflow Studio UI

Updated:

```text
Loom/apps/desktop/src/App.tsx
Loom/apps/desktop/src/styles.css
```

Added Workflow Studio UI:

- explicit `workflowGraph` model derived from YAML
- selected workflow node state
- editable node draft
- `Graph view` card
- dependency/edge chips
- `Node properties` panel
- editable `Node id`
- editable `Uses tool` with registry tool datalist
- editable `Needs`
- editable `With fields`
- `Apply node changes`
- `Add node`
- `Delete node`

The UI keeps YAML as the durable daemon contract. This avoids introducing a
second persistence format while restoring old ArtLoom's visual editing loop.

## Validation evidence

Local validation passed:

```powershell
npm --prefix Loom/apps/desktop run typecheck
npm --prefix Loom/apps/desktop run build
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\tests\test-loom-artloom-parity-contract.ps1
cargo fmt --manifest-path Loom/Cargo.toml --all -- --check
git diff --check
rg -n "NeuroLoom|Neuro" Loom/apps/desktop/src Loom/apps/desktop/src-tauri/src/lib.rs Loom/resources/python scripts/tests/test-loom-artloom-parity-contract.ps1
```

The prefix regression scan returned no matches.

Browser UI check:

- started `npm --prefix Loom/apps/desktop run dev -- --host 127.0.0.1`
- opened `http://127.0.0.1:1423/`
- navigated to Workflow Studio
- confirmed `Graph view`, `Node properties`, and `Apply node changes` were
  visible
- clicked `Add node`
- confirmed the page showed `step-2`, `2 nodes`, and the YAML editor contained
  the new node with `needs: [prompt]`

The dev server was stopped after the check.

## Release evidence

Generated release:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\build-release-exes.ps1 -Apps Loom -VersionId loom-workflow-graph-ui-phase27 -Force
```

Release directory:

```text
release\Loom\loom-workflow-graph-ui-phase27
```

Executables:

```text
loom.exe
loom-daemon.exe
loom-desktop.exe
```

Package:

```text
packages\Loom-loom-workflow-graph-ui-phase27-windows-x64.zip
size = 50070708 bytes
sha256 = aab71988098e42e9a0b8bada641c2cf98dd61c3fb03960fe55412e42fb67b675
```

Formal verification:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify-release.ps1 -VersionId loom-workflow-graph-ui-phase27 -Apps Loom
```

Result:

```text
status = passed
gitHead = d5e9a7567a8b314b9344846c6cf0c21ebb249f8f
gitDirty = false
checksumEntries = 31
```

Release smoke:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-release-local-apps.ps1 -VersionId loom-workflow-graph-ui-phase27 -Apps Loom
```

Smoke summary:

```text
output\smoke\runs\20260612-214123-Loom-42272-f44cf8936c4f4453b4f3a5fbc0dbf1f4\release-local-apps-loom-workflow-graph-ui-phase27-Loom-summary.json
output\smoke\latest\release-local-apps-loom-workflow-graph-ui-phase27-Loom-summary.json
```

Smoke remained green across the previously restored runtime paths, including:

- packaged `loom-desktop.exe`
- MCP marketplace discovery and connection testing
- management CRUD
- embedded Python
- Python Art catalog and execution
- cloud multipart Art node execution
- real OCR
- workflow-backed direct tool execution
- workflow Art node execution
- workflow AHRP execution
- Hook Bridge WebSocket handshake and broadcast

## Non-goals

- This phase does not clone old ArtLoom's exact ReactFlow/AntD implementation.
- This phase does not add `@xyflow/react`.
- This phase does not restore Python source-file read/import helpers; those
  remain a separate audit item after the installed Python Art catalog and
  execution path restored in Phase 24.
