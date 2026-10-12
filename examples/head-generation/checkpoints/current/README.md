# Ear return-fold checkpoint 500

Source-only recovery checkpoint, 2026-10-12. Ear-only partial improvement; user aesthetic acceptance is still pending. Eyelashes are frozen.

Retains checkpoint497 bounded harmonic ear/jaw fairing and lower-pinna forward adjustment. Adds one middle row to the existing open return membrane, preserving its outer and inner boundaries. The shallow crown rises at most 2 mm posteriorly and fades smoothly to zero at its endpoints. The two actual crown planes have separate geometric normal regions while remaining smooth along their length. Root normal blending remains enabled. No filled rear cap or reference geometry is introduced.

## Reproduction
Requires the owner's existing Beaver project, private assets/aster/head_recovery_49/aster_head_editable.blend, authoring/head_generation_guide.md and NPR runtime dependencies. This checkpoint is not standalone. No reference mesh, UV, texture or screenshot is included.
Package with python3 bundle.py --output request.json, then load through the existing native Beaver blender.start workflow. Validate and preview through native NPR. Do not execute pipeline.py directly.

## Verification
277 pure tests passed in 21.557 seconds in the complete working source. The bundled focused modules are only a subset of that suite.
Native generation 12066809-3e75-43b8-b614-ca373bbbd329 completed 2026-10-12T10:13:09.426Z, exit 0, duration 49213 ms; all 31 declared output hashes verified. Native NPR validation at 10:14:22.558Z returned ok:true, errors:[].
Actual GLB500 versus499: all positions and triangles exactly unchanged; only normal sets at 31 ear-region Face positions changed. Body and hair normals unchanged. The preceding498 topology change added 26 physical ear vertices, with both 14-edge root seams retained. The499 crown increase moved 124 ear vertices, maximum 1.196362 mm; outside ear support exact difference zero.
Inspected native500 at180 white,150 white/wire,45 render and both90 render views. The rear return fold is clearer and both side views show no obvious new black corner. Ear shape, texture and reference equivalence still require further refinement. Passing technical checks is not final aesthetic acceptance, merge, deployment or a zero-warning claim.
