# Candidate 94 feedback-batch review — 2026-10-06

This is a generated, inspected development checkpoint, not final aesthetic or universal-template acceptance. Candidate 94 was executed through Beaver's existing task and native tools; no model-service call or Codex task was created.

## Actual execution

- Job eae4e85a-9a17-4c46-bf14-0543c977e0e3; request 7275699c-df9c-4a96-b4fe-528e1d011b54; succeeded, exit 0.
- GLB SHA-256 bf142c47edf3a9309fa8f9393c4cebf8bf1669cec216288f10537a06e8da9bb1.
- Focused Python suite: 27 tests passed. Native NPR character validation: ok, errors empty, output hashes verified. This is not an absence-of-all-warning claim.
- Published Python runtime sources are AST-equivalent to the actual generated copies; source formatting and documentation do not retroactively alter the exported model.
- MiDot, the pinned NPR 1.3.0 package, Body and Hair remain unchanged.

## Feedback implemented through the recipe

- Localized upper/lower lip motion with a fixed lower-chin region. Actual front chin-tip samples move 0 mm versus 15.11 mm in candidate 89. Skin-UV-filtered lower-chin samples have up to 0.709 mm downward / 0.767 mm total motion; do not claim the entire chin is perfectly rigid. Broader geometric masks include oral vertices and are not valid chin-only metrics.
- Neutral lip width reduced and vertical position adjusted; cavity and tongue retained. MouthOpen 0, 0.5 and 1 inspected; neutral 0 restored.
- Ears are shallow, open-backed elliptical membranes with independently authored pigment. The evaluated face and each ear now share 14 root edges, each incident to exactly two faces. Candidate 94 has 54 vertices and 36 quads per generated ear plus local end triangles. This does not establish perfect reference shape matching.
- Upper-head height and depth boundary controls smooth the silhouette. Candidate 93's absolute-depth blend flattened the forehead and was rejected; 94 uses a boundary depth correction while preserving the middle forehead curvature.
- Brow thickness/rise and taper adjusted to sparse semantic observations.
- Eye-pocket rear bodies use constant oval cross-sections; oral rear rings also share an oval section. Front apertures and cavity requirements remain independent; the default mouth is not an open pipe.

## Visual evidence and limits

Native synchronized front render, left-side white, rear white and full-MouthOpen captures were inspected, with Body/Hair off, zoom 1.37 and light -45 degrees. The reference stays neutral when candidate MouthOpen is driven; the capture does not compare both actors' animation. Ears and lips can still need further aesthetic feedback. Hair intersections are outside this frozen-Hair batch. Reference export splits or triangulation artifacts are not copied into the generator.

## Validation and publication gates

The first aggregate project check correctly failed even with exit 0 and five passing tests: an old test preloaded retired NPRLab files absent from the pinned runtime. Its migration tests the current head-review actor, nonmutation, view controls and explicitly preserves rejection of missing full-wardrobe data. Aggregate rerun status is recorded with the checkpoint; focused model validation is not a substitute for it.

GitHub PR 20 remains draft. Earlier checks failed before runner/job steps and a bounded rerun did not resolve that infrastructure symptom; no current-head CI success, external review, merge, release or production deployment is claimed. Approval to initiate external review while CI fails is still pending.
