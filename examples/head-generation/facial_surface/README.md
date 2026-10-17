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
were initially preferred in the internal review, but the user subsequently
rejected case717 aesthetically. It is retained only as comparison evidence;
technical coverage does not establish acceptable eye design.
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

### Illustrated eye-art experiment

`--style-profile emerald_illustrated` is a further opt-in art experiment. Use
`emerald-rounded-eyes.json`, the same surface parameters/bound, and `--blink`.
The aperture is unchanged from case717. The profile tapers the inner upper lash,
sweeps its small child blade along the parent, reduces/tints the pupil, relocates
the key catchlight and groups iris paint into dark upper / colored middle / light
lower regions. An isolated sclera atlas island adds upper occlusion and warm
corners; its UV mapping and ocular ILM classification are changed together.

Native case720 generated and rebaked its SDF through Beaver. Its standalone Face
export passed structural, finite-stream, three-morph and same-job embedded-atlas
checks (8,182 vertices, 10,972 triangles). 69 render/white/blink and 42 supplemental
wireframe/opposite-side/mouth captures were checked against their actual weights,
display modes, source identity and image hashes, then visually reviewed. This is
a technically checked candidate that the user subsequently rejected for an
upturned aperture. It is not aesthetic acceptance. Existing
white-model cheek/temple shading bands and exact tangent-repeat limitations remain.
No hair/body geometry was redesigned; the showcase still uses hidden frozen roles.

The source adaptation is retained in the generated `facial_surface_preflight.json`
under `packager_helpers.semantic_illustrated_art.py`; transformed native sources
are retained alongside the output. No third-party reference images or models are
included in this profile.

### Level-corner rounded-eye comparison

`emerald-level-eyes.json` is a new opt-in comparison, using the illustrated
profile with `--blink`. It cancels the baseline corner rise with
`corner_slope_offset=-0.014`, uses eye width `1.0`, upper/lower arch `1.4/1.0`,
and a restrained main lash with no independent upper tuft. The illustrated main
rail follows the aperture to station 0.96 before a short outer return ending at
1.06. Non-illustrated profile rails retain their previous constants.

Use surface parameters `{"axial_smoothing":0.5,"cheek_roundness":0.05,"orbit_shallowing":0.15}`
and the unchanged 0.0042 m support budget. The candidate's conservative bound is
0.0036192162826032934 m. Lowering orbital adjustment, rather than widening the
budget, makes room for the rounder aperture.

Cases721 and723 improved the neutral contour but failed internal posed visual
review because dark outer-lash fragments remained below the closed eyelid. They
must not be treated as accepted models. Case724 tests anchoring the final generated
main-lash column floor to the closed seam, instead of translating every column
from the analytic aperture. This did not remove the visible fragments: a native
magenta-outline isolation proved they came from exposed backfaces in the outline
pass. Case725 adds an explicit control-map holdout for internal graphic/ocular
atlas islands and binds it to the framework Face outline control. Later native
725/726 comparisons removed the outer fragments; see the controlled follow-up
below for the remaining medial line and acceptance limits. The correction
is enabled only for illustrated eyes without a retained upper tuft; old profiles
and retained-tuft motion keep their previous behavior.

The illustrated profile reuses the existing face-map blue channel for outline
holdout; red and green meanings stay unchanged. By default, skin atlas and ear islands remain
unmasked. This requires the NPR framework's full-holdout fragment discard fix:
zero extrusion width alone does not suppress inverted-hull backface fill. The
map does not hide base-color geometry or disable the whole character outline.

### Controlled aperture and gaze follow-up (726–729)

Use `emerald-soft-aperture-eyes.json` to reproduce the 726 parameter set, or
`emerald-nasal-gaze-eyes.json` for 727. Both use `--style-profile
emerald_illustrated --blink`, surface parameters
`{"axial_smoothing":0.5,"cheek_roundness":0.05,"orbit_shallowing":0.15}`
and `--reviewed-support-bound-m 0.0042`. Supply the chosen JSON as `--eyes`,
a fresh output prefix and the authorized workspace to `bundle.py`. Generate
and validate through the normal Beaver workflow, including SDF rebake and
fixed-camera/lighting neutral, wink, full-blink, oblique and white-model views.
Parameter snapshots are reproducibility inputs, not certified model presets.

726 narrows the aperture and iris, deepens the lower bowl and lightens the
upper stroke. Root review of 69 verified native captures found modest subjective
improvement and no old outer closed-eye triangles. A residual far-eye medial
vertical line remains; setting outline width to zero removed it without a visible
skin gap in that diagnostic. This does not authorize globally hiding outlines.

727 changes only `iris_horizontal_offset=-0.006`: both iris/pupil/catchlight
assemblies move nasally by 1.416 mm. Root review of 69 verified captures found
slight subjective frontal-focus improvement, no old triangles in reviewed winks,
and the same medial line. This is not final consumer aesthetic acceptance.

728 instead changes only `iris_plane_slope_scale=0.85` relative to 726 (offset
and skin mask zero), with an explicitly reviewed 0.0045 m support budget. All
69 captures were verified. The front-eye ROI `[435,400,742,490]` had zero changed
pixels out of 27,630 versus 726. It was not selected as a frontal gaze fix; the
control remains experimental. Actual 726's conservative support bound plus the
0.62462946 mm plane allowance totals 0.00316360192 m. Plane orientation is an
art-support parameter and must not be equated with a semantic visual gaze axis.

729 uses `emerald-compact-spacing-eyes.json` with the same profile, blink,
surface parameters and reviewed 0.0042 m budget as 726. It changes only
`eye_spacing=0.96` versus 726; iris offset is zero, plane scale is one and skin
holdout is zero. All 69 actual captures, hashes and weights were verified, and
root reviewed the three-option full-face comparison and winks. Root prefers
729's whole-face composition to 726/727, subjectively; this is an optional
candidate, **not user-accepted final beauty**. The far-eye thin medial outline
at full blink and white-model shading bands remain. No broader geometric or
posed-clearance certification follows from this preference.

The independent `ocular_skin_outline_holdout` is numeric 0/1, default 0 and
**NATIVE UNTESTED**. It is active only in the illustrated profile and affects
only face-map B inside a bounded, calibration-derived ocular skin ellipse. It
preserves R/G/A, existing island B, nose and ear regions. The plateau (`q<=1`)
is B=1, transitioning to zero by `q=1.08`. With lip correction disabled, the
current shader does not taper partial-B outline width: only the full plateau
discards coverage. CPU mask tests do not establish visual safety or acceptance.
Keep this switch off when comparing translation or plane controls.

`gaze-experiment-evidence.json` retains the exact root review summaries, capture
manifest identifiers and comparison limits. The previously recorded tangent
repeatability, hidden showcase roles and posed-clearance limitations still apply.

Research informed separating aperture, translation and support-plane controls:
[OLM, Flexible Eye Design (2020)](https://history.siggraph.org/wp-content/uploads/2022/08/2020-Talks-Derouet-Jourdan_Flexible-Eye-Design-for-Japanese-Animation.pdf)
supports independent artistic eye components;
[PAniC-3D (2023)](https://openaccess.thecvf.com/content/CVPR2023/html/Chen_PAniC-3D_Stylized_Single-View_3D_Reconstruction_From_Portraits_of_Anime_Characters_CVPR_2023_paper.html)
discusses stylized single-view ambiguity;
[Pejsa et al. (2013)](https://graphics.cs.wisc.edu/Papers/2013/PMG13/eg13-pejsa_preprint.pdf),
[Sweeny and Whitney (2017)](https://whitneylab.berkeley.edu/PDFs/Sweeny_Whitney_Gaze_VR_2017.pdf)
and [Disney eye rigging (2019)](https://studios.disneyresearch.com/2019/05/06/practical-person-specific-eye-rigging/)
motivate separating geometric and perceived gaze. These sources do not validate
our proportions or aesthetic judgments. Reference images remain private and are
not included in the repository.

### Whole-face layout follow-up (730–733)

732 uses `emerald-narrow-layout-eyes.json`: spacing .94 versus 729's .96,
with the original medial correction collar and .0042 m support budget.
733 uses `emerald-balanced-layout-eyes.json`: the same layout with lower arch
1.00 instead of 1.08. These remain optional comparative candidates.

730's .90 lower arch made the lower aperture too flat in root review.
731's .92 spacing with an experimental adaptive medial collar passed CPU
support checks but failed native posed-triangle orientation; it was rejected.
The adaptive collar is not retained, and .92 spacing is outside the supported
parameter range. No geometry orientation or overlap guard was weakened.

Use the native case records for actual capture and aesthetic-review status.
CPU success and native structural validation do not establish aesthetic
acceptance or complete posed-intersection safety.

### Outline follow-up (734–736)

733 remains the user-favored baseline. 734's original optional mask and 735's
temporal extension did not remove the main oblique closed-eye line. Neither was
selected. A native material diagnostic identified implicit mip averaging of the
fragment coverage mask; explicit LOD0 removed the line with the same medial
mask, while retaining the inspected nose and face outlines.

`emerald-medial-holdout-eyes.json` retains the 736 experiment. The medial mask
radius is 1.40 while temporal reach stays 1.08; the disabled default remains
unchanged. Generation and SDF succeeded, but complete native review is pending
[the NPR coverage fix](https://github.com/aiaimimi0920/NPRCharacterFrame/pull/10)
and an approved dependency migration. The package integrity guard rejected a
direct modified dependency, and the pinned file was restored. Do not treat the
new config as delivered or visually accepted. CPU mask contracts do not certify
filtered appearance, all poses, or distant temporal stability.
