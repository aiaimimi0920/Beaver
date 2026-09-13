# Phase 27: Workflow Graph UI Parity

## Goal

Restore old ArtLoom's visible workflow graph editing loop in Loom desktop while
keeping Loom's YAML-backed workflow contract and avoiding a large ReactFlow
dependency port.

## Tasks

- [x] P27.1 Source audit and parity boundary
  - Acceptance: source-backed audit identifies old ArtLoom ReactFlow editor
    behavior, current Loom gaps, and the Phase 27 recovery boundary.
  - Evidence:
    - `docs/loom/analysis/phase-27-workflow-graph-ui-audit.md` records old
      `workflow-editor` sources, current Loom gaps, implementation design,
      browser evidence, release evidence, and non-goals.

- [x] P27.2 Contract-first graph UI requirements
  - Acceptance: the ArtLoom parity contract fails before implementation and
    passes after implementation.
  - Evidence:
    - `scripts/tests/test-loom-artloom-parity-contract.ps1` now asserts:
      - `Graph view`
      - `Node properties`
      - `Add node`
      - `Delete node`
      - `Apply node changes`
      - `workflowGraph`
      - `serializeWorkflowGraphLite`
      - `updateWorkflowGraphNode`
      - `addWorkflowGraphNode`
      - `deleteWorkflowGraphNode`
    - Initial RED failure:
      `Missing=[Graph view]`.
    - Final contract run passed:
      `Loom ArtLoom parity release contract passed.`

- [x] P27.3 Lightweight graph model helpers
  - Acceptance: desktop has graph helpers that can serialize visual node edits
    back into workflow YAML.
  - Evidence:
    - `Loom/apps/desktop/src/services/workflowStudio.ts` adds:
      - `WorkflowGraphNodePatch`
      - `serializeWorkflowGraphLite`
      - `updateWorkflowGraphNode`
      - `addWorkflowGraphNode`
      - `deleteWorkflowGraphNode`
    - The helpers keep node ids unique, rewrite downstream `needs` references
      on rename, and remove deleted node references from downstream `needs`.
    - `npm --prefix Loom/apps/desktop run typecheck` passed.

- [x] P27.4 Desktop Graph view and Node properties UI
  - Acceptance: Workflow Studio exposes a visual graph surface and lets the
    user create, edit, and delete nodes while YAML remains the persisted daemon
    contract.
  - Evidence:
    - `Loom/apps/desktop/src/App.tsx` adds:
      - explicit `workflowGraph` model boundary
      - selected node state
      - node draft state
      - `Graph view`
      - edge/dependency chips
      - `Node properties`
      - editable `Node id`
      - editable `Uses tool`
      - editable `Needs`
      - editable `With fields`
      - `Apply node changes`
      - `Add node`
      - `Delete node`
    - `Loom/apps/desktop/src/styles.css` adds graph node, edge, and node
      properties panel styling.
    - `npm --prefix Loom/apps/desktop run build` passed.

- [x] P27.5 UI smoke, release, and regression checks
  - Acceptance: local UI check proves the new graph surface renders and updates
    YAML; release generation, formal verification, and release smoke pass.
  - Evidence:
    - Browser UI check opened `http://127.0.0.1:1423/`, navigated to Workflow
      Studio, confirmed `Graph view`, `Node properties`, and
      `Apply node changes`, clicked `Add node`, and confirmed `step-2`,
      `2 nodes`, and YAML `needs: [prompt]`.
    - `cargo fmt --manifest-path Loom/Cargo.toml --all -- --check` passed.
    - `git diff --check` passed.
    - `rg -n "NeuroLoom|Neuro" Loom/apps/desktop/src Loom/apps/desktop/src-tauri/src/lib.rs Loom/resources/python scripts/tests/test-loom-artloom-parity-contract.ps1`
      returned no matches.
    - `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\build-release-exes.ps1 -Apps Loom -VersionId loom-workflow-graph-ui-phase27 -Force`
      generated `release\Loom\loom-workflow-graph-ui-phase27` with
      `loom.exe`, `loom-daemon.exe`, and `loom-desktop.exe`.
    - `packages\Loom-loom-workflow-graph-ui-phase27-windows-x64.zip`
      was generated with size `50070708` bytes and sha256
      `aab71988098e42e9a0b8bada641c2cf98dd61c3fb03960fe55412e42fb67b675`.
    - `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\verify-release.ps1 -VersionId loom-workflow-graph-ui-phase27 -Apps Loom`
      passed formal verification with
      `gitHead = d5e9a7567a8b314b9344846c6cf0c21ebb249f8f`,
      `gitDirty = false`, and 31 checksum entries.
    - `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\smoke-release-local-apps.ps1 -VersionId loom-workflow-graph-ui-phase27 -Apps Loom`
      passed. The smoke summary is:
      `output\smoke\runs\20260612-214123-Loom-42272-f44cf8936c4f4453b4f3a5fbc0dbf1f4\release-local-apps-loom-workflow-graph-ui-phase27-Loom-summary.json`.
    - Smoke evidence kept prior restored paths green, including:
      - `mcpMarketplace.connectionTestSuccess = true`
      - `managementCrud.workflowDeleted = true`
      - `pythonArtCatalog.count = 1`
      - `pythonArtToolExecution = "python art saw release installed python art"`
      - `pythonToolExecution.packagedPython = true`
      - `cloudMultipartArtNode.multipartSeen = true`
      - `realOcrImage.fullTextLength = 63`
      - `workflowToolExecution = "script saw release workflow runtime"`
      - `workflowArtNode.success = true`
      - `workflowAhrpProcess.status = "Success"`

## Notes

- Old source reference is read-only:
  `Z:\project\project\ArtNexus\ArtLoom`.
- Phase 27 restores visual workflow graph behavior, not the exact old
  ReactFlow implementation.
- No `@xyflow/react` dependency was added.
- Remaining likely follow-up work is old Python source-edit/import helper
  parity beyond the installed Python Art catalog restored in Phase 24.
