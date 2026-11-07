# Bounded final source-triangle fairing

This is an optional source-generation stage, enabled by the existing
`sampling_surface_fairing` control. Defaults remain OFF. The generator first
constructs the low-budget mesh directly, then adjusts selected positions before
normals, pose checks and export. It never creates a dense mesh to decimate later.

## Contract

- Vertex and polygon topology are unchanged. The selected Face remains at its
  original low triangle budget.
- Work is restricted to continuous front skin. Open boundaries, apertures,
  semantic creases, region seams, nose/mouth support, the mirror plane, underside
  and existing hard folds are pinned. Paired displacements preserve symmetry.
- Proposals follow finite differences of the actual local triangle-dihedral
  objective. Global backtracking and local retreat retain orientation, area and
  minimum-angle bounds. A no-op is reported honestly, not counted as improvement.
- Candidates are quantized to float32 before acceptance. Stored coordinates are
  read back and checked again; reports use those values. The displacement budget
  is 1.5 mm with an explicit 0.1 micrometer storage tolerance.
- UV2 is reprojected from final stored positions using the existing face
  projection. Material UV coordinates are not rewritten. This distinction is
  required for the model-derived SDF fitting contract.
- The normal field is recomputed from the resulting geometry. No unrelated
  analytic normals are substituted to hide geometric folds.

## Actual evidence, 2026-10-24

The retained Faye development candidate has 3,878 complete Face triangles:
`0e296d7518c3200e180c0a5dfd13d0fb5abff566279fd493dfe7d6d363fed2c1`.
178 source vertices moved by at most 1.200020 mm. The selected adjacency objective
fell from 57.5356 to 49.5833; its maximum angle fell from 73.8784 to 62.9749 degrees.
Native strict generation, source packaging, model-bound SDF and 36 source-bound
NPR/white/wire/expression screenshots passed their technical checks.

A specifically examined cheek edge improved from 51.734 to 40.009 degrees.
However, 64 neighboring edges increased by more than 5 degrees, including 14 by
more than 10 degrees. This is curvature redistribution, not pointwise monotonic
improvement. Existing over-80-degree hard folds were pinned, not repaired.

Two additional strict generation compositions retained 3,878 triangles:
blunt-chin/elongated-nose/lip-band and compact-rounded/balanced-nose/line-mouth.
Those runs are bounded numeric/pose checks, not separate visual acceptance.

## Visual status and limits

Independent review of all 36 actual views found a small net cheek improvement,
with a somewhat more visible broad diagonal band below the outer eye. NPR views
show little perceptual difference. Forehead creases, crown ripples and the jaw's
hard transition remain. No new conspicuous contour duplication, cracks or
expression intersections were observed, but this is not a collision proof.

Retain this as a limited optional improvement, not final surface-quality or
product acceptance. Do not equate lower aggregate energy, passing tests or green
CI with a uniformly smoother face. Further work must evaluate real geometry and
images together.

CPU regression coverage exercises the actual implementation on a triangulated
surface: real energy reduction without topology growth, pinning, symmetric
motion, float32 storage, UV2 consistency with material UV preservation,
deterministic repeats, no eligible surface, and rejected invalid inputs.
