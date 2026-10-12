# Connected posterior ear return checkpoint 511

Source-only recovery checkpoint, 2026-10-12. Partial ear-under improvement, NOT final aesthetic acceptance. Supersedes rejected503, whose outward opening shift made the colored band thinner. Rejected510 moved existing roots and produced new frontal dark seams.511 keeps507 original geometry and adds a connected two-row open posterior return. No reference mesh or texture is included.

## Reproduction
Requires the owner's existing Beaver project, private assets/aster/head_recovery_49/aster_head_editable.blend, authoring/head_generation_guide.md and NPR runtime dependencies. Not standalone. Package using python3 bundle.py --output request.json, then load through native Beaver blender.start and validate/preview through native NPR. Do not execute pipeline.py directly. The editable blend is the intermediate authoring snapshot; runtime_morph blend and GLB include post-join return geometry. The recipe reproduces final outputs.

## Geometry and verification
Two added rows extend inward by a smooth bounded width envelope, maximum9mm, with2mm posterior curvature. Original507 vertices and triangles remain unchanged.56 connected faces,108 exported triangles,56 unique vertices added; free back opening remains. Saved winding uses original boundary orientation for both rows. Native assertions reject forward-facing return faces, collapsed faces, non-manifold new edges or moved originals.
279 pure tests passed in21.240s in the full working source; bundled focused tests are a subset. Native job0c819581-307b-46e3-9f33-4db17ced7d95 completed2026-10-12T14:36:22.590Z, exit0,46231ms,32 outputs SHA-verified. NPR validation2026-10-12T14:36:50.429Z ok:true,errors:[]. Both roots retain15 shared manifold edges. Actual507 to511 position audit: zero moved/removed original vertices, body/hair unchanged, Face5710 to5766 unique vertices.

## Visual review and remaining work
Actual front and90-degree render show no newly introduced diagonal ear-under seam.180-degree colored comparison shows a wider continuous return than503.150-degree white and render inspected. Reference skin-band widths and lower opening curve still differ; some lower white-surface faceting also existed in501. Further calibration and owner approval remain. This checkpoint is backup of useful incremental work, not an accepted final model, merged change, deployment, zero-warning claim or completed product. Eyelashes remain frozen.
