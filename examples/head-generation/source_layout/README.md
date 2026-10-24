# Generated-source structure

This package splits retained, generated authoring sources at explicit responsibility
boundaries without changing the frozen recipe files or mathematical controls.

- `pipeline_geometry.py`: geometry creation and containment checks.
- `pipeline_assembly.py`: material/atlas assembly and editable source save.
- `pipeline_runtime.py`: runtime joins, evaluated geometry and complete morph loop.
- `pipeline_export.py`: GLB export, merge and reports.
- `semantic_parameter_curves.py`: the original parameter curve functions.
- Optional `source_mesh_rules.py`: shared paired-pose polygon validity,
  triangulation and bounded source sampling. `ear_surface.py` retains local skin
  and final-triangle fairing, importing the extracted rules.

Pipeline stages execute in their fixed order with the same global frame. They are
ordinary Python files, not ignored files or encoded strings used to evade checking.
The migration record stores original and split source hashes. Byte reconstruction
must reproduce the complete original source, including comments and whitespace.

## Native two-job protocol

`face_components.bundle.build_plan(...)` returns `generation` and
`source_packaging`. The CLI writes the generation request plus a companion
`.source-plan.json` file. Keep the latter outside the project workspace.

1. Submit `generation` through Beaver. It performs one actual model generation,
   executing the four stages in the same frame, with up to 64 declared outputs.
2. After the successful native receipt, call `packaging_request(plan, receipt,
   workspace)` and submit that request through Beaver. This verifies generation
   hashes and writes the five base helpers (plus `source_mesh_rules.py` when declared) and `source_layout_binding.json`.
   It is a source-packaging job, **not another model generation**.
3. Pass both receipts to the composition SDF builder and final verifier. A modular
   report without the second native receipt is rejected. Neither native receipt
   is rewritten or replaced by a fabricated aggregate receipt.

The optional surface split rejects unsupported external globals, shadowed builtins,
dynamic namespace lookup, decorators, annotations and dynamic defaults. Both
packaging and final verification reconstruct its exact original bytes. This is
not a general-purpose refactoring engine for arbitrary Python modules.

Both requests enforce output-count and code-size limits. Prefixes, entry hashes,
helper hashes, the first request's script hash and exact declared output set are
checked. Linked or escaping source paths are rejected before local reads.

The generation job validates the segmented execution path, not an independent
rerun of the retained disk `pipeline.py` launcher. Unit tests exercise the launcher
with synthetic functions; do not describe those tests as native model execution.

Historical source refactoring changes source hashes. Historical image/model
observations remain historical evidence, but do not assert that old complete
artifact-chain verifiers still pass against the changed source files. Current
acceptance must use fresh native generation, source packaging, SDF and captures.

Run contract tests with:

    python -m unittest discover -s examples/head-generation/source_layout -p 'test_*.py'
