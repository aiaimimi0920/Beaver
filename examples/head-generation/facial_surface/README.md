# Facial-surface hypotheses after native672

This folder contains CPU studies and an experimental native-request packager. Frozen coherent672, composed673, and the rejected masked671 are intact.
No likeness, smoothness, continuous bound, or final attachment acceptance is
claimed.

Strict numeric controls (all default0): axial_smoothing0–1,
cheek_roundness0–.15, orbit_shallowing0–.5. They respectively broaden the axial
profile correction, add a low-order transverse component, and contract the
base-to-rim support gap. No identity selector or final mesh restoration exists.

The full cheek correction is retained through normalized lateral q=.65 and
fades to zero at .85. Additions have zero transverse derivative at q=0, with the medial collar now cancelling the inherited center-fold slope when axial smoothing is active. The original ear topology is not
edited by these pure functions; exact evaluated ear connectivity/pinning is not
established.

Orbit support is explicitly independent: raw=base_old+base_delta;
rim=rim_old+base_delta+beta*(base_old-rim_old). Thus the analytic gap contracts
by1-beta. Shared base/rim offsets alone leave that gap unchanged, as in672.
This formula does NOT yet transport an iris or guarantee cavity clearance.

## Current measured hypothesis, not accepted candidate

For the parent-selected target-eye controls, axial.5/roundness.05/shallowing.25:
sampled base displacement≤1.927mm and rim≤1.875mm. The proposed sampled2mm
budget is checked without clamping; this is not a continuous certificate.
Inner corner t=-1 gap8.388→6.291mm; upper arc midpoint5.305→3.979mm;
lower arc midpoint4.595→3.446mm; lower outer edge t=.98 gap.325→.244mm.
These are analytic boundary gaps, not the visible iris-center recess.

Vertical curvature variation improves on four sampled cuts. Transverse variation
rises approximately2–3.7× at h=.5–.7, although sampled extrema counts do not grow.
The candidate must not be advertised as a fairness improvement on this evidence.
Broad intended curvature and localized collar bending require separate analysis.

## Requirements before a builder

- Review 2D curvature/high-frequency residual and collar transitions, including
  center-fold and diagonal cuts. Do not optimize only vertical sections.
- Certify base displacement on the full(h,q)domain using piecewise-polynomial
  bounds/adaptive subdivision. Sample maxima alone are insufficient.
- Bound the orbital gap over the aperture region, including corner neighborhoods;
  fractional-power contour derivatives make a uniform sampled derivative bound
  unreliable. Do not replace the check with a relaxed threshold.
- Fit a common new ocular support frame from rim-boundary samples. Move iris,
  pupil and catchlights together; preserve iris concavity, relative layers and
  planar rear cap. Verify frame-fit residuals against clearances before geometry.
- Establish iris/rim/socket interval clearance and regenerate cavity lengths from
  the new support. A plane fit by itself does not prevent crossings.
- Regenerate shell, lash/brow/fold attachments, oral boundary/back wall/tongue and
  additive mouth deltas from the same field. Validate actual poses separately.
- Inspect final mesh after inherited fairing/subdivision; require exact topology
  and ear-region connectivity checks, rather than inferring them from a zero
  analytic displacement collar.

Run CPU tests with `python -m unittest discover -s . -q`. study.py accepts
explicit JSON controls and eye controls and writes a diagnostic JSON only.

## Reviewed constrained transverse experiment

transverse_fit.py replaces the cutoff with a degree7 Bernstein correction.
Center value/first derivative and outer value/first/second derivatives are fixed;
three free nonnegative bounded coefficients fit the broad target with a global
third-derivative penalty. Coefficient hulls plus exact piecewise-cubic profile
critical points give a conservative numerical continuous base bound (1nm margin,
not formal interval arithmetic). At regularization1e-4 with the same controls,
that base bound is .993474mm. This excludes orbital/component displacement.

The large outer shoulder is reduced: transverse variation ratios at h=.5/.6/.7
are1.021/1.029/1.054 instead approximately2–3.7. However, this candidate remains
unapproved: sampled rim displacement reaches2.0105mm, and a real exterior
lower-cheek shoulder at x=.04m,h=.2278–.2364,q=.5594–.5652 has2.191µm excursion
across2.030mm height, tangent angle−.0929°…+.00075°. A second .0503µm extremum
at x=.06m lies inside the missing eye aperture and must be recorded separately
from shell-skin metrics. Neither small amplitude nor domain separation erases the old count-guard failure.
The reviewed diagnostic exception retains the exterior ripple explicitly; the
original2mm heuristic is not relabeled as passed.

Reviewed ocular contract: fit an affine displacement plane to newrim−oldrim,
using shared upper/lower/corner boundary anchors. Retain original iris local
concavity and pupil/catchlight offsets. Report plane residual and residual change
around the entire aperture, translation and tilt. Check actual iris/limbus
against lids and socket planes. Deep socket rings remain planar at the minimum
new rim minus configured offsets; shared frame motion alone does not establish
clearance. Visible iris-center recess is measured relative to a fitted support
frame, not the hypothetical base surface inside the opening.

## Experimental packaging and support review

The constrained runtime field is now retained in runtime_field.py. preflight.py
fits a common affine displacement plane from both complete rim arcs and bounds
its value at the corners of the full iris/pupil/catchlight rectangle. Both eye
sides are included despite the inherited unmirrored catchlight offset. Lens
concavity and layer offsets are preserved. Existing socket regeneration keeps
rear caps planar; native clearance checks remain mandatory.

For the .5/.05/.25 reference experiment with target670 eye controls, conservative
engineering envelopes are: base .993474mm; aperture rim3.103850mm; analytic
skin/support depth4.139589mm. Actual sampled rim displacement was2.0105mm.
The interval depth envelope includes lower-height cross-section interpolation,
nose/lip relief, orbital branches and the full h0–1,x0–maximum-width domain.
There is no assumed unproved tiny-tail envelope. Floating coefficient/libm
rounding remains an engineering caveat; this is not formal theorem proving.
The ocular plane bound is1.702883mm, translation1.294065mm, tilt1.061768degrees;
maximum sampled residual change is.906661mm. Residuals are diagnostics, not
clearance certificates. Later mesh fitting/subdivision may move vertices.

bundle.py requires --reviewed-support-bound-m explicitly and rejects any
computed envelope above that value. This argument records a reviewed diagnostic
budget; it does not certify collision clearance or artistic quality. Its runtime
rim wrapper also rejects any actually evaluated support call above that budget.
Package --parameters and --eyes as numeric JSON, --workspace as the existing
native workspace for read-only input hashing, --prefix as
assets/aster/general_face_eyes_<case>/, and --output as a request JSON path.
All-zero surface controls produce the same eye recipe control. Do not reuse
previous atlases: use a fresh semantic_eyes smooth-proxy SDF bake after generation.
The old failed count guard and reference ripple diagnostics remain in the report.
Before execution, review the envelope and compare final ears, limbus/socket/lid
clearance, body/hair clearance and mouth poses through authorized native tools.
CPU packaging/tests do not execute Blender, Godot or Beaver.

## Opt-in eye art-direction profile

`--style-profile clean_emerald` applies an explicit original style patch before
numeric eye controls and surface preflight. Omitting it preserves the inherited
baseline. Unknown profile names fail. The profile changes iris/pupil calibration,
colors, lash section and lower-line width, and eyebrow arch; nose, mouth, ear and
lateral face-shape calibration stay unchanged. It does not select a character by
name. Its fully adapted style matches the rendered685snapshot exactly in CPU
comparison. Native cases687/688 subsequently reproduced that Face and its texture PNGs; see the bounded evidence below.

The profile does not repair the remaining white-model cheek flatness. A separate
power18/orbit.15 hypothesis was generated and reviewed in white-model45° view; the lateral bands remained, so it was not retained.
Do not reinterpret the4.2mm correction envelope as a bound on changes to the
underlying calibration itself, or as collision clearance.

## Derived-source preservation and repeat evidence

The checked-in checkpoint remains byte-identical. Lower-liner/iris controls and
the medial-normal blend are applied only to the derived request snapshot. The
normal blend is a named runtime helper with CPU comparison against the reviewed
inline expression. The zero-weight cross-section fast path skips interpolation
only when its coefficient is exactly zero; transition-height behavior remains
unchanged. Dense samples and finite-difference neighbors compare exactly.
Actual native case688 reproduced case685 exactly under joint Face-stream comparison, including morphs and oriented triangles; face_base, face_ilm and face_map PNGs were byte-identical. Case688 eye construction took36.37s versus83.02s in case687, and control-map baking52.47s versus97.22s. These are two observed runs, not a cross-machine benchmark.

The source-speed helper is embedded verbatim in the preflight JSON, itself
covered by the source manifest, so the request retains all sources within the
64-output native limit. It is not silently discarded to fit the limit.

## Optional lower-ear bridge repair

A changed crease calibration exposed a shape-specific assumption: the old
postprocess required one or two initially inverted bridge triangles even when
none needed repair. The derived snapshot now accepts zero repair candidates as
an explicit no-op. A single failing side still requires one unambiguous mirrored
counterpart; asymmetry, ambiguity and more than two candidates still fail. The
later limiting-face pass applies the same pairing rule. Final area, orientation,
local-support and unchanged-outside-scope assertions remain intact.

Native case693 exercised the zero-required branch without removing those checks.
Its reduced crease weights did not adequately improve the white-model cheek
transition, so that style change was not retained. Case694 repeated the existing
689 configuration with this repair logic and produced exactly permutation-equal
joint Face streams and byte-identical face texture PNGs. The immutable checkpoint
source remains unchanged; the adaptation is retained as a hashed packaging
helper inside the preflight JSON.

### Layered emerald eye profile

The separate opt-in `emerald_layered` profile retains the original
`clean_emerald` profile unchanged. It moves the coupled iris layers farther
behind the lid, enlarges the hidden pocket support, increases eye contour
sampling, and uses restrained submillimeter short-leaf depth relief. This broader art
profile also reduces nose-plane pigment strength and footprint; it does not
change nose geometry. Both generated scope fields explicitly report that
pigment change, so it must not be described as an eye-only image ablation.
`emerald-layered-eyes.json` is the explicit numeric eye-control example.

The `short_upper_tuft` control defaults to zero. Enabling it requires
`supplemental_lashes=0` and `lash_contour_follow=1`; it derives one tapered
child outline from the adapted main-band rails and retains only that optional
child. Bounded depth relief is applied by this control even when used directly
without a facial style profile. Every vertex in its root column must remain within 0.8 mm of the actual
parent surface after fitting. Existing closed-volume, front-normal and main
attachment checks remain active. This bounded root check is not an all-surface
collision proof.

Use the existing packager with `--style-profile emerald_layered`, the example
JSON as `--eyes`, and the explicit surface parameters
`{"axial_smoothing":0.5,"cheek_roundness":0.05,"orbit_shallowing":0.25}`.
The reviewed support bound remains 0.0042 m. Do not raise it merely to force an
incompatible parameter combination through. Generated candidates still require
fresh native geometry, texture validation and multi-angle visual review.

### Rounded lower-eye comparison candidate

`emerald-rounded-eyes.json` is an additional opt-in comparison preset, not a
replacement for the layered example or a final character acceptance. It uses
`lower_arch=0.78`, iris width/height `1.04/1.08`, and iris vertical offset `0.002`;
all other eye controls match `emerald-layered-eyes.json`. Use the same surface
parameters and 0.0042 m reviewed support bound above, with `--blink`.

Native case717 passed the existing bound at 0.004099519056779347 m. A trial with
`lower_arch=0.85` exceeded the fixed budget and was rejected. The retained
candidate was reviewed in 69 render/white/blink poses, 42 supplementary
wireframe/opposite-side/mouth poses, and 250 directional-light captures. All
500 captured pane material states matched the requested SDF settings. The
face-review UI probe passed 30 checks. The fuller lower return and larger iris
were visually preferred in the reviewed front and both 45-degree views.
White-model cheek/temple shading bands remain unresolved.

Independent case718 regenerated the same joint geometry, morphs, UVs, normals,
material metadata and embedded images, but **failed exact full-stream repeat**:
one exported tangent component differed by approximately 0.0001. The exact
comparator remains unchanged. A native diagnostic of the saved meshes found
identical expanded oriented corner inputs, with raw tangent computation differing
by at most 1.7881393432617188e-7 before Blender's four-decimal exporter rounding.
Three free/recalculate cycles on the same saved mesh also changed raw tangents
while leaving corner normals unchanged. The underlying native computation cause
is not yet established. This preset
must not be advertised as bit-exact repeatable or fully accepted.

The local same-job standalone Face SHA-256 values were
`18926f0843003697afeb5d474698df3a03bc7be5dc406b3ab73e6dabc3bef28f`
(case717) and
`284767265455745df8eb651c37ccbfe85026a9d8e071504bfc80f844dff75677`
(case718). The native showcase still uses an assembled definition with hidden
Body/Hair roles; these checks do not establish a standalone Face-only NPR runtime.
