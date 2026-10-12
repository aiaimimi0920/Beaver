# Ear membrane checkpoint 501

Source-only recovery checkpoint, 2026-10-12. Ear-only work; final aesthetic acceptance remains pending. Eyelashes remain frozen.

Retains500's shallow 2 mm open return crown and separate geometric normal regions, plus497's bounded harmonic ear/jaw fairing. Changes only the existing ear interior depth-profile sample at u=.70 from .72 to .66. This introduces a slightly more concave interior while preserving the root, outer rim and return-fold coordinates. The change is bounded rather than a wholesale ear replacement. No reference assets are copied.

## Reproduction
Requires the owner's existing Beaver project, private assets/aster/head_recovery_49/aster_head_editable.blend, authoring/head_generation_guide.md and NPR runtime dependencies. Not standalone. Package with python3 bundle.py --output request.json and load via native Beaver blender.start; then native NPR validation and preview. Do not execute pipeline.py directly.

## Verification
278 pure tests passed in 21.238 seconds in the full working source. Bundled focused tests are a subset. Native job fbe6a3b9-ea1c-43b7-8173-94462bfc56f1 completed 2026-10-12T10:29:40.013Z, exit0,49695ms,31 output hashes verified. NPR10:30:50.414Z ok:true,errors:[]. Actual GLB500 to501:141 Face vertices changed, maximum1.589581mm;5696 unique Face vertices unchanged. All vertices outside ear support match exactly, body and hair unchanged.
Native501 white-model views at60/90/150 degrees inspected. Interior adjustment is subtle; the500 return-fold improvement remains visible. Rear opening remains unfilled. Reference equivalence and user aesthetic acceptance are not established. Technical checks do not establish merge, deployment or zero warnings.
