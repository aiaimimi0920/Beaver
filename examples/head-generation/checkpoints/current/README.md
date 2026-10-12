# Connected posterior ear return checkpoint 513

Source-only recovery checkpoint,2026-10-12. Useful partial ear-under correction; final appearance and owner acceptance remain pending. Replaces rejected503 outward shift. No reference mesh or texture included.

## Reproduction
Requires the owner's existing Beaver project, private assets/aster/head_recovery_49/aster_head_editable.blend, authoring/head_generation_guide.md and NPR runtime dependencies. Not standalone. Package with python3 bundle.py --output request.json, load through native Beaver blender.start, then native NPR validation and preview. Do not execute pipeline.py directly. Editable blend is an intermediate authoring snapshot; runtime_morph blend and GLB include final post-join changes.

## Construction
All507 original positions and triangles retained. Two connected posterior-return rows are added to the real skin/ear free boundary. Upper width peaks at10.5mm at normalized height .235, support .11 to .37; lower width7mm with support .01/.055/.115. Only the upper part has2mm posterior depth. Lower rows extend medially at original depth to avoid512's new side silhouette step. Stationary center vertices are reused, avoiding four degenerate exported triangles found in the first513 trial. Back opening remains open. Eyelashes and other separate components frozen.

## Verified evidence
280 pure tests passed in21.023s across the complete working source; bundled focused tests are a subset. Corrected native job6bb5b6da-4a27-4746-8895-9633e651746e completed2026-10-12T15:05:36.321Z, exit0,52813ms,32 output hashes verified. NPR2026-10-12T15:06:05.063Z ok:true,errors:[]. Actual507 to513: original vertices unchanged, Face5710 to5862 unique vertices; body/hair unchanged.304 added exported triangles have nonzero area and backward orientation. Both roots retain15 shared manifold edges. Tests do not imply zero warnings.

## Visual limits
Rear colored-band widths are much closer in the inspected same-camera comparison; ear-root transition and outer contour still differ. Front and90-degree render show no new black diagonal seam; the lower side step from512 is removed. Rear white and wire surface checks remain distinct from final aesthetic approval. This is a recovery checkpoint, not final acceptance, merge or deployment.
