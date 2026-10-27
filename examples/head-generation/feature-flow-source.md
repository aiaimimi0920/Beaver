# Experimental quad source flow

This is an opt-in authoring path, not a visually approved character preset.
The ordinary composition recipe remains the default. No reference mesh, UV,
texture, or private project artifact is included in this change.

## Supported input and execution

Use the six-component composition builder with `sampling_feature_flow=1`.
The versioned contract requires density sampling enabled, 13 shell columns,
detail budget 1, and subdivision, surface fairing and outline refinement 0.
Other profiles fail closed rather than partially applying the adapters.

Call `face_components.bundle.build_plan(..., source_seed=True)` or its CLI
with `--source-seed` and a sampling-controls JSON file. This explicitly selects
three native jobs; the default recipe still uses two. `build_request` rejects
seed mode because it cannot carry the complete receipt-bound plan.

1. Run the emitted source-seed request through the existing Beaver workflow.
2. Preserve its original successful receipt and call
   `source_layout.seed_evidence.finalize_seed_plan` before running generation.
3. Preserve the generation receipt and use `packaging_request` for the source
   packaging job. Do not rebuild a finalized plan or manufacture receipts.
4. Verify the full seed/generation/packaging chain before downstream SDF work.

Full serialized native requests remain limited to 1 MiB. Transport sharing
deduplicates identical readable source literals; it does not compress code,
increase the native limit, or silently introduce another job.
Owned static source outputs, seed files and packaging helpers use explicit
UTF-8/LF writes. A CPU simulation of Windows text defaults verifies byte/hash
stability; this is not a native Windows execution claim. Copied receipts retain
their original bytes.

## Source responsibilities

- `feature_flow_recipe.py` owns adapter order and retained source inventory.
- Feature, mouth, chin, eye and lateral-band modules own their named sampling
  or connectivity changes. The cheek guide preserves the inner mouth aperture and
  ear attachment boundaries and distributes transverse curvature toward the
  nasal region; it does not create a high-valence nose-tip vertex.
- Ear geometry separates main/smooth-cap continuity from creased return walls.
  The fit preserves its fixed source anchors and keeps the existing bounded
  displacement, orientation, diagonal and area guards. The opt-in upper-front
  refinement requires SciPy in the native Blender Python environment; installing
  it on a separate host interpreter is not evidence of native availability.
- Weld ownership captures numeric point attributes and named group weights
  before joining. Only matched skin/oral boundary vertices receive skin donor
  fields. Positions, loop-domain UVs and oral-only points are not overwritten.
  Ambiguous matches or incompatible object axes/scales fail closed.
- Editable skin and ears remain quads. Export chooses a common valid triangle
  split for neutral/open geometry; triangles are an export/runtime concern.

## Replaying retained source

The first generation executes complete inline source. Packaging writes the
separate readable modules and their hash-bound reconstruction record.
For the ordered face/shell/ear families, use
`source_layout.module_layout.execute_ordered_module(entry_path, record, namespace)`.
It validates the entry, ordered helpers and reconstructed source before executing
the verified in-memory snapshots in one shared namespace.

Legacy checkpoint loaders that use a bare `ModuleType` without a filesystem
location cannot directly execute these split entry files. Do not hide this
limitation by bypassing hashes or falling back to unverified source.

## Evidence and remaining work

Early native source iterations on 2026-10-26 generated 920 skin/ear quads. The retained
source refactor was checked against the prior model: all 55 meshes had matching
positions, topology, UVs, point attributes and deform weights, and runtime GLB
bytes were identical. Later source-flow and ear corrections intentionally change
geometry, so that earlier equality is not evidence for those later shapes.

The later cheek guide removed the measured outward turnback and distributed
curvature across two traced bands. Ear source fitting reduced the measured
uncreased upper-cap fold and the nearly straight lower-quad corner. Original
ear-corner UVs and crease ownership were checked separately from new interpolated
split-point UVs. These are bounded geometric findings, not artistic acceptance.

Known remaining issues include a narrow upper-ear cap, the eye outer-frame
appearance and final same-camera expression/runtime review of the latest shape.
Successful generation, source tests, or CI do not certify visual completion,
absence of warnings, deployment, or final product acceptance.

The later public-recipe regression also matched the corrected experimental
model across all 55 source meshes and the complete runtime GLB bytes. Source
packaging reconstruction and the original three-job receipt chain were verified
independently of that geometric comparison.

The subsequent upper ear-to-skin attachment revision keeps the original 40 ear
points, their corner UVs and crease groups unchanged. It moves three attachment
points and one adjoining skin point per side, retaining all source face indices.
Actual saved-mesh checks found the affected attachment's largest 3D corner
reduced from about 155 to 135 degrees, with right-side and right-45-degree
projection maxima reduced from 162.56/165.59 to 139.21/126.71 degrees.
Both six-quad skin patches retain two-face incidence along the shared split
seams; new corner UV interpolation errors were below 3.1e-8.
Native generation, source packaging and opaque source-wire three-view rendering
completed for this revision. This does not certify global self-intersection
absence or final runtime/artistic acceptance. The adjacent front smooth cap,
eye outer-frame and mouth faceting remain unresolved.

The shared oral weld now restores skin-owned crease EDGE fields as well as
POINT fields. Duplicate skin/cavity edges previously made `crease_edge` and
`FacialCreaseGroup` ownership depend on weld ordering. Capture requires a
unique pre-weld skin/cavity boundary edge; restoration requires exactly two
mixed-owner incident faces and a bijective endpoint donor correspondence.
Only these shared edges are written. Native validation in the subsequent
experimental mesh confirmed all 30 shared oral edges retained the skin donor's
crease/group values; this is an attribute-ownership result, not a claim that
the experimental eye or lip shape has final artistic acceptance.

The opt-in eye chart also rounds the shared rectangular support locally with
a compact C1 field, while retaining the original aperture reference and fixed
head rim. Its four radial intervals are redistributed to 0/.25/.5/.75/1.
The upper lip's two internal samples move toward the existing peak and shoulder
with an angle-dependent fade; the lower-lip samples, aperture and outer boundary
remain fixed. This is sampling of the existing depth function, not additional
lip-depth smoothing or an increase of the reviewed support bound.

Actual experimental saved-mesh checks preserved both eye apertures and all
30 shared oral aperture points and their morph deltas. The upper-lip first
radial gap decreased from 13.723 to 4.803 mm. Both lower internal rings remained
unchanged. Eye-frame XZ displacement was about 2.02 mm, but reevaluating depth
on the existing surface produced up to 5.97 mm total 3D displacement; do not
describe this as a 2 mm spatial bound. Source generation and sampled mouth
poses passed their checks, and nine actual engine pose captures were verified.
These checks do not remove the remaining upper-lip crest/valley or establish
continuous-domain collision safety or final artistic approval.

### Local upper-lip depth and outer-ring transition

The opt-in source path now keeps the first upper support ring at its earlier
peak-sampling location, but uses a compact C1 upper-half window for the second
and third rings. One additional closed support ring splits the long outer
span. It does not globally densify the head or reconstruct reference quads.

Depth fairing is limited to internal upper-mouth samples: the inherited
reference and calibration remain unchanged, the total local depth difference
is capped at 5.2 mm, and the additional fairing move at 1.5 mm. All other depth
calls retain their existing reviewed bound (4.2 mm in the checked profile).
These are depth differences, not limits on total three-dimensional vertex
displacement. Non-finite values fail closed. The new lower-half support points
use midpoint depths, and those final interpolated depths are checked against
the original lower-region bound rather than checked only before replacement.

Saved-model checkpoints using these adapters retained the oral aperture and
added 30 source points/quads, producing 950 skin/ear quads. One central 15.192 mm
span became 7.139/8.114 mm. The following outer-ring transition reduced the
measured central-versus-shoulder height difference from 2.964 to 0.176 mm.
Only 28 moving mouth points required new UV and morph values in that step;
fixed oral points and unrelated mouth fields remained unchanged. Independent
81-pose CPU checks did not find degenerate or reversed sampled triangles.

A real nine-pose engine capture was verified for the added-ring checkpoint,
not for every later shape. Subdividing a nonplanar lower quad does not preserve
its triangulated surface exactly: sampled distances reached 0.117 mm near a
mouth corner. Narrow ear regions, concentrated nasal turns and final visual
likeness remain open. None of these measurements, source tests or CI results
constitutes artistic, global-collision or deployment acceptance.

### Rounded outer mouth frame and bounded upper-ear support

The mouth outer frame now uses a compact C1 source-chart rounding field.
Amplitude 0.12 and a broad lateral fade of 0.65 distribute the transition across
nearby cheek columns rather than leaving the next column rigid. The central
nasal strip, eye/ear attachment heights and original inner aperture references
stay outside the field. This changes sampling coordinates, not the analytic
nasal depth function, and adds no vertices or faces. The disabled-strength
path remains exactly the identity. Actual saved-mesh checks found 80 moving
skin points, at most 4.592 mm, with UV changes restricted to those points.
Sampled exported mouth poses passed; two unchanged ear-root points had about
1.2 micrometers difference in one MouthOpen component, so unrelated morph
buffers are not claimed byte-identical.

The upper-ear refinement keeps connectivity and authored UV weights. It holds
the previously repaired three-quad attachment patch fixed while allowing one
upper outer support point a separate 1 mm design limit. The remaining 18 fixed
points and all ear roots stay fixed. Existing free points retain their prior
scale-relative displacement limit; finite-value and float32 post-validation
checks retain the original diagonal, orientation and area gates. The tested
outer support moved about 0.886 mm, without moving the attachment patch.
The narrow cap's maximum required-view angle decreased, but an adjacent front
face became narrower in one view. This is a documented local tradeoff, not a
claim that every angle improved or that the ear matches the reference exactly.

The authoring recipe must retain the ear refinement module and its SciPy
runtime dependency. Source quads remain editable; the exporter automatically
selects tested diagonals. Passing the sampled exported mesh does not establish
that arbitrary retriangulation is safe. Inner-lip support concentration and
final same-camera visual likeness remain separate open review items.

Focused CPU checks, from this directory:

```sh
python -m unittest discover -s source_layout -p 'test_*.py'
python -m unittest discover -s semantic_eyes -p '*flow.py'
python -m unittest discover -s semantic_eyes -p 'test_weld_ownership.py'
python -m unittest discover -s semantic_eyes -p 'test_ear_attachment.py'
python -m unittest discover -s face_components -p 'test_feature_flow_recipe.py'
python -m unittest discover -s face_components -p 'test_generated_source_size.py'
```

The ownership fixtures are CPU mocks, not a replacement for native attribute
and pose checks. The Python effective-line counter treats multiline data/code
strings as code and is conservative; it does not replace the repository's
`npm run check:effective-lines` Rust scanner. Record an unavailable scanner as
unrun/blocked rather than treating these tests as its result.

## Design references

- [Autodesk: Modeling the Head](https://download.autodesk.com/global/docs/softimage2014/en_us/userguide/files/face_modeling_ModelingtheHead.htm)
  discusses facial edge direction and deformation. Software-specific budgets
  are not adopted as this recipe's acceptance criteria.
- [Disney Research: Sketch-Based Generation and Editing of Quad Meshes](https://la.disneyresearch.com/publication/sketch-based-generation-and-editing-of-quad-meshes/)
  motivates controllable patch boundaries and edge flow, not a unique facial
  template or automatic recovery of original quads from triangulated assets.

Shape, edge direction, connectivity, shading and deformation must be evaluated
separately. Quad-only authoring by itself does not fix any of the latter four.

## Joint first-support-ring refinement

The source recipe now solves the first mouth support ring jointly on the current
semantic rays. It does not use historical vertex IDs or a saved solution table.
The aperture, midline endpoints, second and third rings, and outer boundary stay
fixed. Upper first-ring fairness uses 0.33, while pinned midline points retain
the previous value and the other upper rings retain 0.35.

The optimizer reads a pure pair of current depth and the unchanged inherited
reference. Every accepted point is checked again through the strict depth
function before any vertex is written. The existing 4.2 mm raw-depth gate,
upper-local 5.2 mm total-reference bound and 1.5 mm additional move bound remain.
Ring gaps, projected orientation, both 3D diagonals and float32 output are checked.
An infeasible solve raises rather than partially updating the ring.

In the native process comparison, only 28 first-ring points changed. Upper
shoulder height above the intended ridge decreased from about 6.422 to 0.618 mm.
The mouth-corner turn increased from 29.905 to 30.885 degrees, within the explicit
one-degree limit. This does not fix all outer transitions: turns of about 40.6
and 44.5 degrees remain. Sampled export pose checks passed, but these checks and
the source comparison are not a claim of final visual acceptance.

## Depth-only tangent refinement

A subsequent bounded solve adjusts only the upper first and second support-ring
depths. It leaves XY stations, the aperture, midline, corner, third ring and outer
boundary fixed. The allowed increment is intersected against both the unchanged
inherited-reference bound and the total additional-fairness bound; the increment
itself is never substituted for the total fairness amount.

The native comparison changed 20 points by at most about 0.609 mm. Two radial
turns decreased from about 47.03/46.13 degrees to 40.89/39.72 degrees. UVs and
morph deltas were unchanged. These are limited improvements, not a claim that
the remaining lip ridge is smooth or visually accepted.

The optimizer retains its double-precision constraints. Final stored-geometry
checks compare the float32 candidate against the float32 baseline, not a mixed
precision baseline. A native regression fixture records an unchanged lower-ring
point whose baseline quantization alone changed the measured turn by 0.001564
degrees. The explicit circumferential numerical comparison tolerance remains
0.001 degrees; depth, spacing and other geometric limits are not expanded.

Solver convergence can differ across Python/NumPy/SciPy environments. The
runtime still rejects a non-converged solve without writing vertices. The
storage-precision regression replays a captured feasible native solution while
running the real constraints and final guards; separate tests exercise the
optimizer and rejection path. This test separation is not a claim that
cross-platform optimizer convergence has been solved.

## Bounded paired seam depth field

A separately authorized aperture/cavity stage follows the two support solves.
It leaves every XY station and all support rings fixed. The depth field is
0.75 mm times (1 - (abs(x)/mouth_half_width)^4)^2: upper and lower seam samples
at the same x receive the same increment, tapering smoothly to zero at corners.
The first two cavity boundaries inherit the moved aperture; deeper cavity rings
remain fixed. This does not reallocate source samples or manually triangulate.

The stage checks both double and float32 geometry against the original aperture
4.2 mm reference bound, never the upper-support 5.2 mm exception. It commits only
after finite displacement, original-reference, orientation and quad checks pass.
The user-authorized seam displacement ceiling is 1.5 mm; the current peak is
0.75 mm. Native source comparison changed 48 aperture/first-inner-ring points,
with all XY and other static geometry unchanged. The maximum original-reference
seam displacement was approximately 1.6512 mm; UVs remained unchanged.

This is a limited process candidate, not final visual acceptance: the central
side-view lip step is slightly shallower, while the remaining mouth flow issue
persists. A uniform 0.75 mm translation was not retained because it worsened the
corner radial turn. The corner-fixed field avoids that specific regression but
increases some seam circumferential turns by up to about 2.55 degrees.

Actual automatic export triangulation changed four cavity quad diagonals. All
81 sampled mouth states stayed nondegenerate and unflipped, but the minimum
normal alignment fell to about 0.06064 in the Open+Spread+Pucker combination.
This must not be replaced by a more favorable test using older diagonals.
Two static ear attachment points also lost approximately 0.0012 mm of inherited
MouthOpen delta during source assembly; the ear source code and static geometry
were unchanged. Existing welded attribute ownership is a suspected cause, not
a proven repair. These observations prevent claims of bitwise morph stability,
global collision freedom, or a fully resolved lip surface.

The native review also captured nine hash-bound engine states, including neutral,
each individual mouth control, blink checks, and Open+Spread+Pucker at right45.
This confirms actual runtime loading and those applied controls, not exhaustive
self-contact exclusion. Keep the prior baseline available for visual comparison.

## Joint upper support sampling and depth

A later semantic patch stage follows the retained paired seam field. It jointly
adjusts the first and second upper support rings along their existing rays and
in depth. Unlike the earlier depth-only solve, the central internal supports
may move in height/depth while their midline x stays pinned. The aperture,
corner rays, lower lip, third ring, outer boundary and polygon count stay fixed.

The optimization balances radial turns and support-gap disparity. It checks
all affected source-quad edges, both triangulation diagonals and station order,
then independently checks stored float32 spacing, order and tangents. The
original depth reference and its raw 4.2 mm, upper-local 5.2 mm and total extra
fairness 1.5 mm bounds are unchanged. First radial turns may not increase; the
0.001-degree numerical tolerance applies only to circumferential comparisons.
Failed solves or final guards leave the source vertex list unchanged.

A separate cheek sampling candidate extends only the upper height fade of
mouth-frame rounding from 0.25 to 0.5 in normalized coordinates. This reaches
the retained chart row at height 1.6045 and ends before 1.608. The lower fade,
rounding amplitude, lateral fade, mouth outer boundary and eye frame remain
unchanged. Chart-plane diagnostics are not native surface acceptance: both
changes require fresh authored geometry, export-diagonal pose checks and
front/right45/right90 source-wire comparison before retention.

Native intermediate evidence: joint support reconstruction changed 30 upper
support vertices, reducing the central radial turn from 27.45 to 20.26 degrees.
Original total depth stayed below 5.152 mm and total additional fairness below
1.5 mm. Source UV/motion changes stayed on those vertices. Its white-wire views
show a modestly softer transition, not elimination of the lip step.

The following cheek-fade reconstruction changed ten skin vertices only. Actual
3D longitudinal turns at lateral stations .44/.55 decreased from 43.79/36.50
to 39.20/32.91 degrees; adjacent edge-length ratios improved from 2.32/2.15 to
1.87/1.81. The target transverse-chain turn increased from 3.38 to 4.62 degrees,
and a local edge length increased 14.54 percent. The mouth solver's ten-percent
edge-change guard must not be misreported as a whole-face result. Eye, ear and
mouth geometry/motion stayed fixed in this cheek-only comparison.

Each intermediate model's actual new export triangulation (4492 triangles)
passed 81 sampled mouth combinations without flipped or degenerate triangles.
The prior cavity combination's minimum alignment near 0.06064 persists. These
checks neither establish collision freedom nor certify reference-level likeness.

## Joint C1 cheek chart redistribution

The retained raw13 feature-flow profile also provides a compact two-row chart
correction between source heights 1.599 and 1.614 and lateral stations .18 and
.78. The two interior rows, 1.6045 and 1.608, share signed transverse curvature,
longitudinal-turn and spacing objectives. Piecewise smoothstep interpolation
has zero correction and zero first derivative at all four support boundaries.
It is added after the prior complete chart, including orbital rounding. Fixed
semantic coefficients are used; no final mesh vertex IDs, reference mesh
coordinates, runtime optimizer or additional topology are introduced.

Native comparison changed exactly 20 skin points. The complete mouth aperture,
cavity, support rings and outer boundary stayed fixed. Target transverse turns
improved from 4.618/1.595 to 3.296/1.558 degrees, with neighboring edge ratios
improving from 1.485/1.572 to 1.368/1.414. Every measured signed transverse turn
kept its sign without amplification. Two longitudinal turns improved from
39.195/32.910 to 36.006/29.808 degrees. These metrics support a local improvement,
not a claim of final likeness or reconstruction of the reference author's quads.

The largest displacement was 1.173 mm and the largest original-reference depth
offset was 0.3423 mm. Local edge-length change ratios ranged from .857 to 1.244;
these are sampling redistribution tradeoffs, not a claim of a global ten-percent
edge bound. All 4492 actually exported triangles passed the 81 sampled mouth
combinations. The known two ear-root MouthOpen deltas returned from zero to
approximately 0.001192 mm during assembly, with unchanged static ear geometry.
Do not claim bitwise attribute ownership stability or complete collision freedom.

## Lower lateral cheek redistribution

A second C1 chart field covers heights 1.570–1.599 and lateral stations
.44–.94, leaving the mouth frame and outer head boundary fixed. It changes
16 bilateral skin samples, not their quad connectivity. Native comparison
measured longitudinal turns 25.292→21.363 and 12.652→11.349 degrees, and
neighboring length ratios 1.649→1.336 and 1.821→1.519. Transverse spacing
ratios improved 1.756→1.625 and 1.818→1.670; the outer transverse peaks
near 39.317/37.105 degrees remain, so the long-band problem is not solved.

Maximum point movement was 1.18195 mm; maximum original-reference depth
offset was 0.69947 mm. Local edge-change ratios were .84127–1.22695, not a
uniform ten-percent bound. Mouth, eyes and ears stayed fixed, including
mouth motion; UV and morph changes were confined to the same 16 samples.
All 4492 actually exported triangles passed 81 sampled mouth combinations
without flips or degeneracy. Actual opaque wire views support retaining this
small improvement, not final reference-level acceptance. Mouth-to-cheek
patch connectivity and lower-jaw strip allocation still require review.

## Lower mouth-frame connection

The second density transition in the uppermost graded-chin strip now rotates
one shared quad edge through its authored transition point. The change is
expressed in source row/slot roles before mirroring and compaction; it is not
a final vertex-ID patch. Native comparison kept all 2538 vertex positions,
all morph coordinates and per-vertex UV sets unchanged, while replacing four
quads bilaterally. The boundary is identical and no nonmanifold edge appeared.
Each side replaces two valence-five points and one valence-three point with
one valence-five point. The new connection is 11.76 mm versus 13.13 mm before.

The 4492 actually exported triangles passed 81 sampled mouth combinations;
four triangles changed. Fixed vertex positions do not imply identical surfaces:
local sampled depth interpolation changed by up to about 0.386 mm. One original
corner's fan-normal spread rises from 3.04 to 6.83 degrees. Actual front and
right45 opaque-wire views support a modestly more direct lower-mouth-to-chin
transition, not final likeness. The upper frame and broad lower-cheek strips
remain unresolved; no global artistic or collision acceptance is claimed.

## Bounded lower ear-root source stations

Only the opt-in slender-oval, first-pass source-budget profile applies the
normalized u1/u2 correction at the first nonendpoint ear row. It runs before
the existing complete source-fit gates, retaining the original 3.999999-unit
radial budget in double and stored precision. Incremental movement is bounded
to one millimeter; worst touched-quad twist must improve, the short segment
must not shrink, chain turning must not regress, and local area/orientation
bounds remain enforced. Other ear profiles retain the previous path.

Native comparison changed exactly four ear-interior points, at most .999847 mm.
The other 2534 vertices, source topology, region labels, skin-root chains and
ear silhouette boundaries stayed fixed. Worst diagonal warp decreased from
59.619 to 44.357 degrees; the short segment grew from 2.329 to 2.500 mm and
maximum chain turning decreased from 88.919 to 79.999 degrees. Neighboring
quads redistribute warp; this is not a uniform per-face improvement.

Deform weights, six morph deltas, crease ownership, UVMap and DetailUV stayed
unchanged. Four points' fourteen UV2 corners updated by source projection.
All 4492 actual triangles passed 81 sampled poses without flips/degeneracy;
changed triangles had no intersections against the full Face in those samples.
Actual white-wire views show less lower-root bunching without new visible
cracks, but an angular side-cheek transition persists. This is local retention,
not a universal ear-profile or complete artistic acceptance claim.

## Joint outer-frame and cheek sampling

The protected mouth is still constructed with the retained .12 chart through
all internal lip solvers. A separate source chart subsequently resamples 18
half-face grid stations, before compaction and mesh/UV/morph generation. It
combines upper frame rounding and two-row cheek compensation without using
final vertex IDs. Internal solver evidence is explicitly pre-outer-resample;
final quad and pose acceptance must inspect the new outer boundary.

Native comparison changed 36 peripheral points, at most 2.717914 mm, matching
the continuous chart's float32 predictions exactly. The other 2502 points,
including all 120 mouth seam/support points and the cavity, stayed fixed.
Their morph coordinates stayed fixed too. Topology/region ownership is unchanged;
UV changes are confined to 146 corners on those 36 points. Original-reference
depth offset is at most .39653 mm. The actual new triangulation replaced eight
triangles and passed all 4492 triangles across 81 sampled mouth combinations.
Strict local segment/triangle tests found no crossings; coplanar contact is
outside that particular proof.

The .44/.55 longitudinal spacing ratios improve 1.509→1.207 and 1.775→1.535.
Two projected columns become consistently directed arcs rather than slight
S-shaped reversals. This redistributes curvature: transverse peaks increase
3.296→9.758 and 1.558→6.707 degrees, and a final-support aspect ratio rises
2.65→3.30. Actual opaque-wire views support retaining a local flow improvement,
not final reference-level similarity. The outer-mouth corner and coarse-to-fine
transition remain visible and require further source patch work.

## Whole mouth/cheek source-patch redistribution

A subsequent stage now redistributes the second/third lip supports, their outer
frame and two adjacent front-skin edge neighborhoods. The aperture, first lip
support and semantic nasal core remain fixed. A half-face harmonic solve uses
mirror-Neumann height weights and fixed midline x. One uniform step stops at
90% of the first projected convexity boundary. Current source-quad bilinear
coordinates transport the bounded residual and rim fields over the unchanged
analytic raw surface. No saved model, exported triangle or final vertex ID is
used to create the patch. Only front faces enter the sampling domain; rear
surface projection overlaps are excluded explicitly.

This is an explicit whole-patch migration scope with an 8 mm trial ceiling,
not an assertion that it satisfies the preceding 3 mm frame-stage limit or
older local +/-10% edge-length constraints. Original 4.2 mm raw/default,
5.2 mm upper-support and 1.5 mm additional upper-residual depth budgets remain.
Both stored float32 and source coordinates must pass depth, convexity,
orientation and diagonal guards before the transaction writes any vertices.
UV and morph fields are then regenerated by the existing source pipeline.
The earlier lip/frame reports describe their intermediate stages, not the
final whole-patch geometry.

Native trial 217 changed 132 points (69 half-face stations, six on the midline),
with maximum station migration 7.25798 mm. Aperture, first lip support, nasal
core and cavity coordinates and morphs are unchanged. Source topology and
region membership are unchanged; UV/UV2 differences affect 532 corners on
those 132 points, while DetailUV stays fixed. Two unchanged left-ear junction
points also lose an existing 1.19209 micrometre MouthOpen export delta, with
a corresponding source-attribute change. This is recorded as a small ownership
tradeoff, not advertised as an ear fix or strict global morph preservation.

The actual new 4492-triangle runtime replaces 80 triangles and passes 81
sampled mouth combinations without flips or degeneracy. Final GLB positions,
triangles, three UV sets and six sparse morph targets match the runtime values.
Original-reference depth difference is at most 1.592031 mm and upper residual
1.044355 mm. Strict interior tests of 338 affected triangles over 81 states
find no nonadjacent crossings; this is not a complete coplanar/contact proof.

Actual opaque-wire front, right45 and right90 images support retaining the
reduced box corners and lower-lip crowding without a visible new lip step or
nasal silhouette damage. Outer-cheek narrow bands remain; this is not final
artistic acceptance. Fresh SDF and nine-state Godot runtime validation are
separate from these geometry/export checks.

## Five nasal-side flow source trial

The next trial replaces a bounded source-quad patch rather than bending five
old horizontal rows through unrelated nasal stations. The five normalized
reference paths are analysis guides inferred from a triangulated reference,
not a claim to recover the original artist's quad topology. No reference mesh
or historical generated point/face IDs are retained in the production recipe.

Current semantic iris fan centers, the actual source nasal peak, and five
outer source-grid anchors determine the guide placement. A deterministic
intersection/notch/narrow-gap rule selects each replacement disk. Boundary
valence, crease groups, functional-feature exclusions and mirror correspondence
must match before any edits occur. Every new point samples the containing
current authoring quad, using the same lineage for numeric point and corner
fields. Existing boundary coordinates and crease values remain unchanged.
Blink and mouth shape keys are recomputed by the ordinary downstream pipeline;
old blink deltas must not be interpolated onto the new vertices.

This stage targets the current bilinear authoring-quad surface, not an arbitrary
runtime triangulation or an extrapolated procedural residual field. The visual
trial permits at most 1 mm sampled surface deviation and quad angles between
10 and 170 degrees; original source depth guards also remain enforced. This is
not final artistic acceptance, nor a claim that earlier 0.55 mm research targets
were achieved. Native mesh, UV/field, pose, collision and opaque-wire checks
must pass before retaining the candidate.

The generated shell retains one shared module frame, split into ordinary,
hash-bound geometry, domain, core, binding and mesh responsibilities for replay.
A large direct runtime compile literal may share the existing readable source
seed with the retained recipe. The full seed hash is verified before parsing;
only a single string assignment is read with literal evaluation, never executed.
The original retained recipe bytes and original generation AST are preserved.
Native request/output limits are unchanged. Prior frozen source packages and
their matching replay helpers remain untouched.

Native trial 219 realizes this patch exactly: 122 old faces are replaced by
278 quads, adding 240 sampling points and removing 84 obsolete interior points.
The complete editable Face has 2694 points and 3063 faces; its runtime has 4804
triangles. Retained geometry and source-boundary crease data are unchanged.
Three corner UV sets, point fields, deform weights and all six actual runtime
shape keys match the independently recomputed expected values exactly.
Actual runtime triangles pass 729 sampled pose combinations; the native
continuous blink/open gate has minimum ratio 0.162833. For 552 new nasal
triangles against the full Face, 99 sampled poses have no strict intersections,
excluding shared-vertex, coplanar, tangent and edge-only contacts.

The three actual opaque-wire views show the intended five upward nasal flows,
but their outer annulus still has an abrupt density transition and concentrated
turns. This is a retained intermediate improvement, not final visual approval.
