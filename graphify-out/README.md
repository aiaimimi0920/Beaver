# Beaver code graph

Built with Graphify 0.9.44 on 2026-09-14 (Asia/Shanghai), from working-tree
code at commit `958fc2cda6124d63962733387dd6b01f551434e9`.

The index is ready for local Graphify MCP and CLI queries. Extraction used
local AST parsers and Cargo workspace introspection, with no semantic LLM
extraction. The existing change in
`resources/skills/blender-production/references/anime-npr-character.md`
was preserved and is outside the code-only scope.

## Outputs

- `graph.json`: searchable code graph; 3,446 nodes and 7,867 relationships.
- `graph.html`: interactive visualization, approximately 3.3 MB.
- `GRAPH_REPORT.md`: graph hubs, 193 named communities, cohesion scores,
  cross-community relationships and suggested questions.
- `coverage.json`: exact coverage, files producing no nodes and limitations.
- `diagnostics.json`: read-only structural checks on the persisted graph.
- `benchmark.json`: Graphify's synthetic retrieval context-size comparison.
- `manifest.json`: per-file extraction fingerprints for incremental updates.
- `finalize.py`: local helper to refresh names, reports and visualization.

Community names describe the dominant source file and subsystem. They are
navigation hints; mixed communities do not establish ownership boundaries.
Community cohesion ranges from 0.052 to 1.0, so not every detected community
is tightly connected. Native Rust, legacy Electron and UI prototype code
are all present; always check the returned source path.

## Coverage and limits

Graphify detected 388 code files. Nodes reference 366 source files:
173 Rust, 110 TypeScript, 52 TSX, 7 JavaScript, 7 MJS, 6 Python,
4 PowerShell, 5 JSON and 2 TOML files.

The remaining 22 detected files produced no nodes. Most are data-only JSON
files; the workspace-root `Cargo.toml` also has no source node. The Cargo
pass separately extracted two crate nodes and one dependency edge. See
`coverage.json` for the complete omission list. Successful extraction does
not guarantee that every symbol in a represented file was recognized.

The detector did not classify 47 tracked `.gd` files as code. Godot scenes,
resources and shaders, CSS and HTML also lack AST coverage here. Documentation
and images were deliberately skipped. Generated/dependency directories such
as `node_modules`, `target`, `dist`, `release`, `output` and `graphify-out`
were excluded by detection or repository ignore rules. Code prototypes and
some tracked schema files remain in scope.

Graphify's name-based sensitive-file filter skipped the design-reference
files `tokens.json`, `tokens.css` and `neuro-design-tokens.png`. Their contents
were not added to the graph.

Use FastCtx search for unsupported files and exact IPC method strings.
The graph does not reliably connect string-routed IPC, dynamic executor
factories, process stdio protocols or all cross-language boundaries.
The graph is undirected: a shortest path establishes proximity, not the
direction of execution. Validate important relationships at the returned
`source_file` and `source_location`.

## Query through MCP

Always pass the project explicitly so another repository's graph is not used:

```json
{
  "project_path": "C:\\Users\\Public\\nas_home\\Beaver",
  "question": "ValidationView",
  "mode": "bfs",
  "depth": 2,
  "token_budget": 1500
}
```

Send this object to `mcp__graphify__query_graph`. Other useful tools are
`get_node`, `get_neighbors`, `shortest_path`, `get_community` and `graph_stats`.
Use exact symbol names or a few English code terms for focused results.

Validated entry points:

- Task scheduling: `Scheduler`, `native/core/src/scheduler.rs:48`;
  the worker handoff is at line 261. Native execution starts at
  `Execution`, `native/core/src/executor.rs:34`, with `run` at line 95.
  The graph does not establish the complete scheduler-to-executor chain.
- Legacy Codex process integration: `LocalExecutor`,
  `src/core/executor.ts:25`, and `CodexRpc`, `src/core/rpc.ts:11`.
  Keep these Electron paths distinct from the native implementation.
- Asset-task UI: `useAssetState`,
  `src/ui/asset-task/use-asset-state.ts:7`; `assetTask.state` is called
  at line 25. Use an exact-text search to locate its native handler.
- Validation: `ValidationView`,
  `src/ui/validation/ValidationView.tsx:27`, calls `useValidationQuery`
  at line 42 and `useValidationMutation` at line 47.
- Validation hooks: `src/ui/validation/useValidation.ts:8` and line 67;
  string-based API dispatch occurs at lines 38 and 74.

## Query through PowerShell

Run from the Beaver repository root:

```powershell
rtk proxy graphify query "ValidationView" --budget 1500 --graph graphify-out\graph.json
rtk proxy graphify query "Scheduler Execution" --budget 2000 --graph graphify-out\graph.json
rtk proxy graphify query "useAssetState" --budget 1500 --graph graphify-out\graph.json
rtk proxy graphify path "ValidationView()" "useValidationQuery()" --graph graphify-out\graph.json
```

## Refresh after code changes

Run the following from the Beaver repository root. `extract` uses the existing
manifest and graph for incremental extraction; `finalize.py` consumes that
graph and regenerates the derived artifacts without LLM calls.

```powershell
rtk proxy graphify extract . --code-only --max-workers 4 --cargo
if ($LASTEXITCODE -ne 0) { throw "Graphify extraction failed" }
$python = (Get-Content -LiteralPath graphify-out\.graphify_python -Raw).Trim()
rtk proxy $python graphify-out\finalize.py
if ($LASTEXITCODE -ne 0) { throw "Graphify finalization failed" }
```

The interpreter marker is machine-local. On a different machine, install
Graphify and run `finalize.py` with the Python interpreter that owns its
`graphify` package; the marker will be refreshed automatically.

This setup does not install a background watcher or change global MCP
configuration. Refresh explicitly when source code changes. Keep `--code-only`
unless intentionally expanding the scope to semantic document extraction.

## Verification

Graphify MCP returned the final named graph and the `ValidationView` call
neighborhood. CLI query and shortest-path commands completed successfully.
An independent read-only pass also verified scheduling, asset UI and validation
entry points against source locations.

Persisted-graph diagnostics found zero missing endpoints, dangling endpoints,
self-loops or exact duplicate edges. This diagnostic cannot reconstruct edges
already lost or merged during extraction/build; it does not prove complete
source coverage. The initial build deduplicated two exact nodes.

The benchmark used a 205,053-word code corpus and reports an average estimated
query context of 8,133 tokens. This is a synthetic context-size comparison,
not measured model billing, retrieval accuracy or end-to-end latency.

`npm run check:effective-lines` passed with unchanged historical oversized
files reported as legacy debt. `finalize.py` passed Python syntax validation
and a real finalization run; this README passed the repository's Prettier
formatter. `npm run format:check` found pre-existing formatting issues in 20
untouched files under `src/shared`, `src/ui` and `tests`; details are in
`format-check.log`. Those files were not reformatted as part of indexing.
Application runtime acceptance is outside this indexing task.
