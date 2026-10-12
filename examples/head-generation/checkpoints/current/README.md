# Ear-to-jaw curve checkpoint 490

Source-only recovery checkpoint, 2026-10-12. This does not replace the existing candidate188 recipe or imply final aesthetic acceptance.

The candidate490 recipe changes only the connected ear/posterior lower-face depth. It preserves front width, height, disconnected eye/lash/mouth geometry, hair and body. The prior symmetric candidate489 bump created a visible side-view kink and was rejected. Candidate490 uses a 6 mm bounded asymmetric smooth profile: normalized height .035 to .38, peak .11, with a longer upper taper and connected ear membrane deformation.

## Reproduction

Requires the owner's existing Beaver project, its original assets/aster/head_recovery_49/aster_head_editable.blend, authoring/head_generation_guide.md, and installed NPR runtime dependencies. These private source assets are intentionally not bundled. Reference meshes, UVs, textures and screenshots are not included. This checkpoint is not a standalone character package.

Run python3 bundle.py --output request.json to package the exact recipe, then load the request in the existing Beaver native blender.start workflow. Do not execute pipeline.py directly. Validate the output definition through the native NPR workflow and open the native comparison preview.

## Evidence

272 pure tests passed locally in 18.892 s before generation. The two included focused test modules pass with python3 -m unittest discover -p 'test_*.py'. They are a small reproducible subset, not all 272 tests.
Native generation job d8e1eeeb-7d1c-411e-99a3-af547b84a7e3 terminated successfully at 2026-10-12T05:28:41.495Z, exit0, 46203 ms. All31 declared output hashes verified. Native NPR validation at05:29:47.944Z returned ok and errors[].
Actual GLB comparison against488: face unique vertex count5670 unchanged;246 vertices moved more than0.1 micrometre; zero changes outside the authorized support. All transverse/height coordinates remain identical; body2565 and hair3110 unique vertices unchanged. Fourteen shared ear seam vertices per side retain two-face incidence.

Reviewed actual runtime at -90, -75, -150, 180 and +90 degrees, including render, white and wireframe views; front MouthOpen0.5 also checked. The rejected489 kink is absent in490, and the annotated ear-to-jaw curve is smoother. Some reference/candidate differences remain; the user has not accepted the final aesthetics. Eyelashes are intentionally frozen.

Hosted CI, merge and deployment status must be checked separately. Local tests and native validation do not prove zero warnings or final delivery.
