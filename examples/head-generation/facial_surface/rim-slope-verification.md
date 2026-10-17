# Local rim-slope experiment — 2026-10-17

This is an opt-in geometric control, not a final aesthetic acceptance.
The retained appearance before this experiment is candidate 744.

## Motivation and exclusions

Matched native comparisons found no clear aesthetic gain from changing only the
iris plane (747) or increasing orbital shallowing (748). Both completed native
generation, matching smooth_proxy SDF and 84 verified pose/view captures. Neither
replaced 744. Additional game-reference imagery showed that far-eye compression
also occurs in real 3D game scenes; it is not by itself evidence of a defect.

The new control changes the shared rim depth field while preserving frontal x/y
aperture coordinates. It keeps the original calibration as the displacement
reference, refits the full ocular plane, and includes the added skin/aperture
movement in the existing reviewed budget. No checkpoint mesh is modified and no
clearance or posed-triangle threshold is relaxed.

## Native candidate 749

- Eye controls: same as 744, including iris_plane_slope_scale=1.
- Surface controls: axial_smoothing=.5, cheek_roundness=.05,
  orbit_shallowing=.15, orbit_rim_flattening=.15.
- Reviewed support budget: .0042 m, unchanged.
- Computed conservative support bound: .003557675647475569 m.
- Generation: 8b129b22-b513-49e0-9da5-e69687b53e52, succeeded.
- GLB SHA-256: f997fd7dfda0cf4cb6e6a27094799f0405fee78ee46303426e260b6f467625a7.
- Matching smooth_proxy SDF: 85ae551a-09bb-4809-8a6c-33ef9c41463d, succeeded.
- Native capture manifest: blink-1792254987-43232, 84 images. Image SHA-256,
  read-back morph weights and unchanged source-before/source-after hashes verified.
- Matched 744 baseline: blink-1792252749-0111, 84 images.
- Continuous posed orientation minimum: .013943538158509572 over 10,980 triangles;
  worst blink=1, mouth=0. This does not prove nonintersection.

Frontal and oblique render, white-model, partial/full blink and 30/60-degree eye
closeups were inspected. No obvious new gap was seen in the reviewed views, but
that is not a comprehensive collision certificate. Visual selection remains
separate from the geometric/control implementation.

## CPU checks

33 facial-surface tests and 104 semantic-eye tests passed. Added regressions cover:
zero/default identity, signed endpoint motion and unclamped extrapolation; unchanged
reference calibration; full ocular-plane delta; additive skin/aperture bounds;
actual packaged native rim/depth agreement; low-budget and out-of-domain movement
rejection; and preservation of the 64-output packaging contract.

Independent image review found only subtle changes and no sufficiently clear
rendered aesthetic advantage over 744 at the tested strength. Candidate 749 did
not replace 744. The control remains opt-in/default-zero; this checkpoint retains
a verified experiment and its constraints, not a new default appearance.
