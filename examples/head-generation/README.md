# Reproducible structure-first head recipe, candidate154

This batch follows the owner's147 review. It calibrates both hollow ears, the posterior chin-to-ear return boundary, the three eye-adjacent ribbons, lower aperture curvature, and independent eye-layer construction. The accepted independent eyebrow, frontal silhouette, nasal ramp, central division and small chin follow remain regression targets.

Workflow: natural-language specification → explicit semantic parameters → Beaver generation → native MiDot/NPR white and wireframe inspection → render and motion checks. The generator never loads reference meshes, UVs, pixels or connectivity. Separate read-only diagnostics distinguish true independent iris/pupil/catchlight sheets from imported UV-split components; raw component count is not a trustworthy layer count.

The iris is an original shallow annular sheet around a separate recessed pupil, with a separate pupil accent and catchlight. Lower aperture curvature is independently adjustable without changing corner anchors. The upper lash ribbon uses a coherent shallow sloped plane after native white-mode inspection rejected a per-vertex projection heuristic. Attached peaks, a tapered eyelid fold, and the lateral liner are distinct from the preserved eyebrow.

Ears remain open sheets with a wider inward return, anterior lower-root calibration and independently authored fold pigment. The posterior jaw uses a connected inward return strip rather than narrowing the accepted exterior. Its end fades out before the ear-root interval. Body/Hair and the pinned MiDot/NPR runtime are unchanged.

69 pure tests pass; generated source hashes match. Native validation passes and actual multi-angle white/wire/render views and MouthOpen0/.5/1 have been inspected. See review-results.md for exact evidence and project aggregate status. Owner aesthetic acceptance remains pending. Keep the existing PR draft until applicable gates pass; no release, deployment or full-character/wardrobe acceptance is claimed.

Run unittest discovery in this directory. bundle.py packages a request for Beaver blender.start and never runs Blender itself.
