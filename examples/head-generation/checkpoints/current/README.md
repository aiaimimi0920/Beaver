# Ear-to-jaw harmonic fairing checkpoint 496

Source-only recovery checkpoint, 2026-10-12. Partial ear-region improvement; final aesthetic acceptance remains pending. Eyelashes remain frozen.

The existing 490 posterior ear/jaw recipe is retained. A bounded length-weighted fairing removes the residual S bend in the free posterior skin edge. Its displacement is extended through the adjacent skin patch with an inverse-edge-length harmonic solve, fixing the outer support to zero. This replaces rejected abrupt two-ring/five-ring falloffs that introduced visible white-model creases or bright patches. Fairing is capped at 6 mm and restricted to normalized height .06–.38 and posterior depth above .18. No topology is added or removed.

## Reproduction
Requires the owner's existing Beaver project, private assets/aster/head_recovery_49/aster_head_editable.blend, authoring/head_generation_guide.md and NPR runtime dependencies. No reference mesh, UV, texture or screenshot is included. This is not standalone.
Package with python3 bundle.py --output request.json, then load in the existing native Beaver blender.start workflow. Do not run pipeline.py directly. Validate the generated definition through native NPR and open the comparison preview.

## Verified evidence
275 local pure tests passed in 22.272 s. The three bundled focused test modules are a small reproducible subset, not the full 275-test suite.
Native generation f34e6898-f7f5-4726-b412-383e5b81caa6 completed 2026-10-12T08:35:30.076Z, exit 0, 51752 ms; all 31 declared output SHA values verified. Native NPR validation 08:35:41.225Z returned ok with errors[].
Actual GLB against 490: 150 face vertices moved over 0.1 micrometre; maximum nearest displacement 0.005999989 m. Face unique vertex count remains 5670; body 2565 and hair 3110 unchanged. Outside the bounded ear/jaw support exact position difference is zero.
Inspected native front render, +90 render, +110 white/wire/render, +150 white, 180 white and -110 white. Posterior edge is smoother, and the abrupt falloff artifact is reduced. Fine ear shape/wrap and reference silhouette differences remain; this is not final user acceptance. Technical success does not establish aesthetic equivalence, zero warnings, merge or deployment.
