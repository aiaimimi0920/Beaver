# Runtime lateral-width reversal: scoped repair

This is a retained development fix, not whole-face visual acceptance. The PR
remains draft. No model, private reference image, or generated texture is
included in this source change.

## Reproduced defect

A source-bound orthographic capture was mapped back to final native triangles.
The editable cage widened upward continuously at the observed lateral stripe,
but the final runtime mesh did not. A native in-memory replay of the existing
editable model recorded the join/weld and eleven subsequent ear/sidewall stages.
The first reversal appeared in `ear_width.fit_front_width`, not in the earlier
source fairing, source profile, join, or local relaxation.

The envelope requested about 4.242 mm outward movement at one lower vertex,
clamped it to 3 mm, and applied 2.713 mm after weights. Its upper neighbor was
outside the envelope support and did not move. The original 2.199 mm upward
width increase became a 0.514 mm decrease. The upper fade occupied only about
3.529 mm of height. Area and normal-orientation checks accepted these valid but
visibly bulging triangles.

A controlled ablation (134, from the 125 baseline) omitted only this envelope
call. It removed the measured local reversal and reduced the outer-cheek bulge,
but did not remove the broader white-model horizontal band. Permanent deletion
of the envelope was therefore not adopted.

## Repair

`source_width_guard.py` retains the existing envelope target and area gate.
After the existing per-vertex area backtracking, it scales the accepted lateral
step uniformly to avoid newly reversing an originally widening triangle chart.

For Blender coordinates `(x, depth, height)`, let `normal` be the unnormalized
edge cross-product normal. The half-width slope at fixed depth is
`-side * normal.z / normal.x`. Because this step changes only x,
`normal.x` is fixed and the numerator is affine in the displacement scale.
Each previously widening chart therefore supplies a linear upper bound.

- Only polygons touched by this step are considered.
- All three vertices must originally lie on the same side of the symmetry
  plane, and their depth/height projection must have nonzero area.
- All vertex triples are considered, covering both quad diagonals and possible
  vertex-only ngon triangulations. This is conservative for concave polygons.
- Existing narrowing is not forced to widen; this is not a global head-shape
  monotonicity rule.
- Exact rational predicates operate on stored float32 coordinates. A bounded
  numerical search repairs rounding at the limiting step; no aesthetic angle
  or distance threshold is disguised as a proof.
- Final stored coordinates are checked again. Original area/noninversion gates
  remain; any area fallback restores the original positions and is reported.
- Ear-root reports are updated only after the final limited geometry is stored.

The report records protected charts, proposed reversals, analytic/applied scale,
rounding work and area fallback. The stage-local guarantee does not replace
final-mesh, seam, deformation, collision or visual checks.

## Verified development evidence

Native candidate 135 retained 4,264 whole-Face triangles. The guard protected
405 charts, prevented 16 proposed reversals, retained scale 0.6451663473 and
maximum movement 1.903944 mm, without an area fallback. The exact scale differs
from the earlier 133 diagnostic because the source profile differs; 135 uses
the original 125 controls, not the rejected profile or normal-weighting trials.

Final GLB SHA-256:
`c1258eaa297f7db74fe3a3605f59530f74b87f56efc0596070fd384e4200f8ad`.

Independent read-only final-mesh checks found no negative 0.1 mm increments in
the declared local y=1.571..1.592 m section at four depths, on either side.
This sampling is a final-output diagnostic, not a continuous global proof.
The former two shelf triangles changed upward pitches about +7.03/+6.65 degrees
to about -5.86/-6.19 degrees. Sixteen distinct ear anchors matched final stored
positions exactly, both six-edge seams had two incident faces per edge, and
all 4,264 GLB triangles matched the native runtime mesh geometrically.

Separate source packaging, model-bound SDF and nine source-bound actual
render/white/wire captures completed. The outer-cheek transition is locally
improved; the broad lower-cheek light/dark band, mouth facets and overall
artistic acceptance remain open. Earlier mouth-open crease warnings are not
claimed resolved by this change.

The retained source increment also includes the previously verified 125 fixes:
closed topology-derived aperture ownership (rather than a coordinate box),
closed-cycle validation, and geometry-consistent shared chin seam normals.
The latter changed normals without changing positions, UVs, colors or the six
morph-position streams in its isolated comparison.

## Focused checks

From `examples/head-generation/semantic_eyes`:

- `python3 -m unittest test_aperture_ownership test_chin_normal_join test_width_guard test_width_guard_adapter test_budget_policy -q`: 32 passed.
- `python3 -m unittest test_aperture_share test_density_layout test_density_medial_normals test_detail_budget test_source_cage_fairing test_polygon_validity test_triangle_fairing test_source_neutrality -q`: 35 passed.

Guard tests include mirrored/reversed winding, both quad diagonals, ngons,
coordinate-axis/unit/origin changes, near-singular projections, 200 random
quads and an actual float32 rounding-boundary case. Adapter tests preserve the
original gate and anchor updates and reject an unexpected source contract.

`git diff --check` passed. The official `npm run check:effective-lines` was
attempted locally but could not run because Cargo is unavailable. Exact-head
CI still has to establish that check; local focused tests are not all-CI or
zero-warning claims.

The three facial-surface packaging suites (`test_upper_warp_bundle`,
`test_angular_bundle`, `test_lower_shoulder_bundle`) passed four tests with all
three new helpers retained in frozen provenance. Duplicated explanatory prose
was shortened without changing arithmetic or checks; the conservative native
frame is 1,048,178 bytes, below the unchanged 1,048,576-byte limit.

A fresh native regeneration (136) from the retained checkout, including the
complete frozen helper provenance, produced the exact same GLB SHA-256 as 135.
This verifies the retained implementation against the reviewed geometry and
morph output. It was not described as a second SDF or capture run.

The first exact-head CI found one longer public-recipe prefix exceeding the
native frame by 205 bytes. Additional duplicated docstrings were shortened;
AST comparison after removing docstrings confirms no executable change. The
existing public source-size regression now passes both almond and segmented
compositions, with the original frame and source-size checks unchanged.
