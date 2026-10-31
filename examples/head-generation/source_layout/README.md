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

## Optional native source-seed preflight

With `source_seed=True`, one or two seed jobs precede the generation/packaging
protocol below. Schema one keeps the runtime source in the seeded bundle;
schema two writes the bundle and runtime as two ordinary Python files in one
native job. If their combined readable request still exceeds the unchanged
one-MiB limit, schema three declares two independent native requests, one per
file. Each complete request must pass the same size/output checks. Schemas one
and two remain supported; no compression or limit increase is used.

For schema three, execute both ordered `source_seed_jobs` and pass their two
original successful RPC receipts to `finalize_seed_plan` before generation.
The verifier independently checks each frozen script hash, exact declared
output/path/hash, current source bytes and receipt identity. The jobs, paths and
receipts must be distinct. Missing, swapped, reused or altered receipts fail
closed; journal copies and synthetic aggregate receipts are not authoritative.
Generation and packaging must also be distinct from both seed jobs. Exact
source/AST restoration remains part of the existing execution contract.

Packaging preserves the original receipt bytes as `source_seed_receipt.json`
and `source_seed_runtime_receipt.json`, and binds both identities through the
report and final verifier. This is two seed jobs plus one generation and one or more
packaging jobs, not multiple model generations. Faye234's actual schema-three pair
passes the independent receipt/source verification. The source chain proves
provenance and reproducibility checks, not visual or engine-pose acceptance.

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


When the retained layout needs more than 64 packaging outputs, prepare it with
`pack_request.py --batch`. The output is an ordered JSON list of ordinary native
requests. Prepare all batches before executing the first one. Execute each separately
and retain every untouched original receipt.
Pass the receipt list to `verify_packaging`, or repeat
`--source-packaging-receipt` for `sdf_request.py`. Every batch rechecks the same
frozen generation hashes; verification rejects duplicate jobs, overlapping or
missing outputs, changed sources and broken seed provenance. Do not synthesize
an aggregate native receipt. This batching does not rerun model generation.
