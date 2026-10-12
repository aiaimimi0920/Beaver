# Ear-under connection checkpoint515

Source-only partial recovery checkpoint,2026-10-12. Reference-equivalent appearance and owner acceptance remain pending. No reference mesh or texture included. Eyelashes remain frozen.

## Reproduction
Requires the owner's existing Beaver project, private assets/aster/head_recovery_49/aster_head_editable.blend, authoring/head_generation_guide.md and NPR runtime dependencies. Not standalone. Package with python3 bundle.py --output request.json, load through native Beaver blender.start, then native NPR validation and preview. Do not execute pipeline.py directly. Editable blend is intermediate; runtime_morph blend and GLB include final post-join changes.

## What changed
513 added two connected open posterior-return rows, upper10.5mm at height .235 with support .11 to .37, lower7mm with support .01/.055/.115. Only the upper row has2mm posterior offset. Stationary center vertices are reused, preventing degenerate triangles.
515 preserves that return construction and shifts the connected posterior ear-under patch outward by at most3mm to reduce an abruptly inset transition. Height support .13 to .29, peak .22; posterior-depth support starts .05 and reaches full weight .12; lateral activation abs(x) .025 to .06m. Shared-root positions are updated consistently before adding return rows. Other components and front-side positions outside this bounded patch remain unchanged. The entire original mesh is NOT unchanged in515.

## Verified evidence
281 pure tests passed in19.940s in the complete working source; bundled focused tests are a subset. Native job0ba56768-fad6-4b3a-88d9-9bfc0704314d completed2026-10-12T15:28:28.190Z, exit0,51139ms,32 output hashes verified. NPR2026-10-12T15:29:47.644Z ok:true,errors:[]. Actual513 to515:182 unique Face vertices changed, maximum nearest displacement2.999403mm;5862 unique vertices unchanged in count. Outside compact posterior height support exact position error0; body/hair unchanged.Native tests reject inverted/collapsed faces and broken shared ear seams. Tests do not imply zero warnings.

## Visual review and limits
Inspected actual front0 and rear150/180 render,150 white and wire. Ear-under colored transition is less abruptly inset; the examined front/oblique views show no newly introduced black diagonal seam. Rear colored-band width and outer curve are closer, with residual lower-lobe shape differences. Fine ear-root shape remains subject to manual aesthetic review. This is incremental work and backup, not final product acceptance, merge or deployment.
