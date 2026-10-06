# Semantic head-generation calibration

This is a recoverable development recipe, not an accepted character asset or a finished universal face generator. It follows specification → explicit parameters → Beaver generation → native MiDot comparison. Current executable recipe: candidate 123 (in-progress calibration).

## Inputs and execution

Restore the user's project checkpoint containing project.godot, authoring/head_generation_guide.md, and the frozen assets/aster/head_recovery_49/ baseline with its editable Blend, GLB, definition and Hair textures. Project/reference assets and runtime binaries are deliberately not published here.

Run python3 examples/head-generation/bundle.py --output /tmp/head-recipe.json to package a request. The bundler does not launch Blender or call any model service. Load the request into the existing authorized Beaver workbench, execute blender.start and poll the returned job to a real terminal result. Outputs are creation-only under assets/aster/head_recovery_123/. Change both bundle.py and pipeline.py consistently for a new candidate; an existing output is not permission to overwrite it.

Use Beaver's npr-character validation and native MiDot review. The companion examples/head-review scripts provide synchronized render, white, wire, side/rear and candidate-only MouthOpen views. Restore MouthOpen to 0 after testing. Technical validation is separate from visual acceptance.

## Current capabilities and boundaries

- Explicit case proportions and sparse semantic cross-sections; generator never loads reference meshes, UVs or textures.
- Real center/side edge-crease profiles, evaluated ablation, and geometry-consistent normals.
- A continuous quad facial cage with chin return bands rather than a visible chin fan.
- Shared curved neutral lip seam, real oral cavity and thick tongue, localized MouthOpen and a fixed lower-chin region.
- Simple oval eye-pocket rear sections and original independent concave iris patches with uniform quad sampling.
- Open-backed shallow ear membranes derived from actual evaluated face boundaries. Runtime welded root edges are checked for exactly two adjacent surfaces.
- Independently authored brow and face/iris/ear pigments, with separate geometric and material controls.
- Upper-head boundary height/depth controls with broad smooth falloff; absolute-depth flattening in candidate 93 was rejected and replaced by a boundary-depth correction in 94.
- Face-only GLB replacement preserves frozen Body/Hair primitives and the original binary prefix. Engine and pinned NPR package remain unchanged.

The chronological authoring brief records each calibration amendment; later amendments supersede earlier case values. Some assembly coordinates and material layout remain project-specific. Parameter effectiveness requires actual geometry/visual checks, not merely a static key read. Do not claim a universal beauty template from this one case.

## Tests and review

Run python3 -m unittest discover -s examples/head-generation -p 'test_*.py'. These pure checks do not replace real Blender execution or native review. See review-results.md for the actual checkpoint evidence and remaining scope. Optional exporter warnings and CI annotations are not claimed absent. Publication, CI, external review, merge and final aesthetic acceptance are separate stages; no production deployment or release is implied.

## Current feedback batch
The user explicitly accepted the center facial shading division visible in108. Later iterations retain it.111 removed the thick lower-ear black wedge in actual side rendering.118 rounds the lower-ear silhouette;119 shortens the original ear pigment pattern and keeps the lobe clear. Ear resemblance and green-marked cheek/jaw curves remain under review. Successful generation and pure tests are not final visual acceptance.

Ear cross-sections use a positive bounded slope variation, with evaluated shared face roots and a thin reverse skin membrane. The generator reads no reference assets. The latest project aggregate explicitly validated120 (7 tests / 57 assertions); 123 has native validation and focused tests but has not yet run that aggregate.
