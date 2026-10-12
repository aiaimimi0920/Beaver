# Ear-to-jaw harmonic fairing checkpoint 497

Source-only recovery checkpoint, 2026-10-12. Partial ear-region improvement; final aesthetic acceptance remains pending. Eyelashes remain frozen.

The existing 490 posterior ear/jaw recipe is retained. Candidate497 additionally reduces lower-ear posterior sweep from .045 to .035, allowing the lower pinna to sit slightly farther forward. A focused regression bounds the additional membrane change to 1.2 mm and preserves root and lateral width. The original394 test fixture remains unchanged in value, alongside the new497 test. A bounded length-weighted fairing removes the residual S bend in the free posterior skin edge. Its displacement is extended through the adjacent skin patch with an inverse-edge-length harmonic solve, fixing the outer support to zero. This replaces rejected abrupt two-ring/five-ring falloffs that introduced visible white-model creases or bright patches. Fairing is capped at 6 mm and restricted to normalized height .06–.38 and posterior depth above .18. No topology is added or removed.

## Reproduction
Requires the owner's existing Beaver project, private assets/aster/head_recovery_49/aster_head_editable.blend, authoring/head_generation_guide.md and NPR runtime dependencies. No reference mesh, UV, texture or screenshot is included. This is not standalone.
Package with python3 bundle.py --output request.json, then load in the existing native Beaver blender.start workflow. Do not run pipeline.py directly. Validate the generated definition through native NPR and open the comparison preview.

## Verified evidence
276 local pure tests passed in 22.289 s. The bundled focused test modules are a small reproducible subset, not the full 275-test suite.
Native generation a0fb6b21-108d-48b9-b494-c8c9c420cf94 completed 2026-10-12T08:53:57.430Z, exit 0, 52912 ms; all 31 declared output SHA values verified. Native NPR validation 08:54:38.481Z returned ok with errors[].
Actual GLB against 496: 234 face vertices moved over 0.1 micrometre; maximum nearest displacement 0.000930342 m. Face unique vertex count remains 5670; body 2565 and hair 3110 unchanged. Outside the bounded ear/jaw support exact position difference is zero.
Inspected native497 front and +90 render, +110 white and 180 white. The preceding496 was additionally checked at +110 wire/render, +150 white and -110 white. Posterior edge is smoother, and the abrupt falloff artifact is reduced. Fine ear shape/wrap and reference silhouette differences remain; this is not final user acceptance. Technical success does not establish aesthetic equivalence, zero warnings, merge or deployment.
