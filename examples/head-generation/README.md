# Reproducible head recipe, iteration 132

Natural language constraints → explicit semantic style and topology parameters → Beaver blender.start → native MiDot/NPR validation and same-camera comparison. No character-name prompt dependency or reference mesh/texture input to generation.

132 replaces the full-area 0.2 mm reverse-ear duplicate with a single wrapped membrane, a shallow cup and a wider inward rim return with an open inner edge. Non-ear geometry remains at 131. Reference ear connectivity and silhouette are read-only diagnostics, not imported generated geometry.

Local pure tests: 54 pass. Beaver generation succeeded. Native NPR validation passed; side and rear white-model views inspected. Overall visual acceptance remains pending. Latest full project aggregate passed at 120; later source tests and native asset validation do not replace it. 133 side-profile calibration is a separate in-progress candidate.

## Reproduce
Run Python unittest discovery in this directory. Use bundle.py --output request.json to package the recipe, then submit through Beaver blender.start. Do not run Blender out of the workflow. Body and Hair primitives and the pinned framework/runtime are preserved.

## Boundaries
PR remains draft. Hosted CI previously failed before runner allocation. No merge, production deployment, release, or final artistic acceptance is claimed.
