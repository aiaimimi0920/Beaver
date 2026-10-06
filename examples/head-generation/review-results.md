# Candidate102 feedback-batch checkpoint — 2026-10-06

This is a reviewable generated iteration, not final aesthetic, full-wardrobe or universal-template acceptance.

## Actual execution and checks
Beaver job0584a02e-8d16-4a7c-87e8-df1f3ea6d2ba, requestb2e00f4d-6900-41cf-8680-be626d1ff61b succeeded with exit0. GLB SHA-25649ead2ae5baebe6b907e78bbbdeb1ff795097373452e1a52527df527b3913e95. Standard NPR validationd3d5b12a-3184-4f00-9822-af1762594da2 returned ok, empty errors, no engine errors and verified definition/model plus pinned framework hashes. Broader dependency-hash verification is not claimed.36 pure recipe tests pass; runtime source AST equivalence is checked separately. Author cage gates report finite vertices, zero degenerate faces and no reversed front/chin-bottom/chin-side faces. Body/Hair and the original GLB binary prefix remain frozen. Optional Draco/color-export warnings remain.

## Changes from user-reviewed94
- Original visible white catchlights:25 vertices/16 quads per eye, following the independent concave iris at a small surface offset. The reference's separate white-mapped surfaces informed the mechanism; no reference mesh, UVs or pixels enter generation. White points visibly survive normal front view and light variation.
- Posterior eye-pocket length0.030H rather than0.065H, transition setback0.004H rather than0.020H, iris concavity0.014H. The iris rim is not globally pushed forward. Left/right sides and oblique views were inspected; no comprehensive all-view collision guarantee is claimed.
- Ear membranes use a rounded transverse profile and a stable authored outer-depth curve independent of root-depth waviness. Each has50 vertices and33 quads plus local end triangles. Each ear keeps13 true shared root edges, incident to exactly two faces. Shape/detail remain subject to user visual review.
- Lower jaw-to-ear return uses bounded width inset(max10% of local half-width) and additive posterior-boundary displacement. It preserves cross-section curvature and fades below the actual eye patch's lower boundary. This is not an exact recreation of the reference's entire side/back topology.
- Neutral lip seam, cavity/tongue and localized MouthOpen retained. Actual front chin-tip displacement remains0; skin-UV-filtered lower chin samples move up to1.077mm downward /1.164mm total. Broad geometric masks include oral vertices and are not valid chin-only metrics.

## Rejected trials and regression evidence
96 failed the unchanged bottom-face orientation gate.97 failed the unchanged front-face orientation gate. They are failed generation trials, not delivered models.98–100 were generated but exposed flattening/creases in oblique white views: an absolute target depth compressed existing cross sections and reached eye-patch boundaries.101 corrected the operation to boundary displacement;102 additionally removes the side-view S-shaped outer-ear wave. No gate was weakened to pass the trials.

Native102 inspected: normal front/render; left/right render sides; right-oblique white; left-oblique runtime wire; rear white; MouthOpen0/.5/1; light-45/-46.52 and+68.76. Captures are real native viewport PNGs with state JSON, not synthetic renders. Runtime triangles are not authoring quad topology. Default front/render/closed mouth restored. NPR shadow matching, overall proportions, ear/lip aesthetics and full wardrobe remain open; the candidate is not declared visually approved.

## Remaining completion gates
The project integration test is being retargeted to102 through Beaver's file workflow, then aggregate validation must be rerun. GitHub PR20 is draft; previous commits failed CI before runner/job steps and no current-head CI or external-review success is asserted. Source publication, durable backup and final user handback are separate steps. No production deployment or release.
