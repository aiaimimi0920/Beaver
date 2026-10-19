# Independent eye presets

Keep the provisionally accepted [795 baseline](../facial_surface/face-baseline-795-prompt.zh-CN.md)
and its parameters unchanged. A different eye shape is an explicit selection with
its own prompt, complete parameter snapshot, and downstream processing. No implicit
replacement of the baseline and no cascading a preset onto already deformed geometry.

## Experimental almond preset

- ID: `almond-balanced-796`
- Intent: [杏仁均衡](almond-balanced.zh-CN.md)
- Numeric recipe: [almond-balanced-796.json](almond-balanced-796.json)
- Current native evidence: generation and receipt-bound SDF succeeded; 84 captured
  views have matching source/image hashes and requested expression weights. Neutral,
  oblique and closed-eye comparisons were inspected. The owner provisionally described this as acceptable; this is a subtle almond
  candidate, not a universal aesthetic acceptance.

`bundle.py --preset almond-balanced-796 --prefix assets/aster/general_face_eyes_796_replay/
--workspace WORKSPACE --output REQUEST.json` packages a request only. Execute it through
Beaver's native Blender tool, then pass the successful generation receipt and the
preset's complete `eye_parameters` to `semantic_eyes/sdf_bundle.py` with its declared
shading policy. The original authoring inputs are still required. Use the normal
NPRCharacterFrame/MiDot validation and same-camera neutral, oblique, white-model,
and blink captures. CPU tests do not establish visual quality.

The selector checks hashes of all three preserved 795 files and refuses the baseline
model directories (795 and 796). Its request-output path must also be outside retained recipe directories. Reusing an existing non-baseline prefix still follows the existing
packager's expected-hash overwrite checks. Future presets need their own authored
configuration, prompt and verification. The catalog preserves the initial seven alternatives and separately lists
reference-refined candidates. Selecting a candidate does not overwrite the baseline.

The `recipe_commit` records the geometry recipe used for the recorded candidate;
it does not silently check out that commit. Use that code revision for historical
reproduction. If the underlying recipe changes, regenerate and verify a new candidate.

Tests: `python -m unittest discover -s examples/head-generation/eye_presets -p 'test_*.py'`

## Slender-flat candidate 797

The independent [prompt](slender-flat.zh-CN.md) and [parameters](slender-flat-797.json)
retain the original iris, eyebrows and face. The optional initial side-lash guide
follows the selected aperture, then accounts for the upper join's actual height.
Its initial depth uses the surface rather than endpoint-chord shaping; later
seating and final guide stages remain active. The diagnostic report explicitly
states that initial chord shaping was not applied. The 4mm support/final-guide,
12mm support lookup and root connectivity checks remain enforced.

This candidate separately uses 0.70 side-sheet blink following, while the legacy
default remains 0.85. Main eyelid closure is unchanged. Native continuous triangle
orientation remained positive (minimum 0.0624564); 16 closest-parent root bands at
five blink weights stayed below 0.8mm (maximum 0.2231mm). These are not a continuous
collision proof or all-root checks. 84 source-bound captures were verified and
neutral/oblique/closed-eye comparisons inspected. Owner acceptance is pending.

For 797 the base code revision plus explicit override hashes identify the tested
recipe. They are evidence metadata, not automatic checkout or hash enforcement
by the selector. Preserve the native source archive for exact investigation.

## Controlled comparison scope

This first preset study deliberately shares one face, iris, eyebrows, camera and
lighting. That isolates aperture shape; it is not a claim that these exact eyebrow
or iris settings fit every future character. Different faces, stronger settings
and new combinations require their own native validation and visual review.

Flat-upper and flat-lower names mean relatively shallow curves, not mathematically
straight edges. The angular candidate changes upper shoulders only; it is not an
entire polygonal aperture. These overlapping design families are not a universal
classification of all anime eyes.

## Eight-style catalog

[Catalog](catalog.json) lists the retained 795 round-open baseline and seven
independent alternatives: 796 balanced almond, 797 slender-flat, 798 soft downturned,
799 sharp upturned, 800 shallow-upper/round-lower, 801 arched-upper/shallow-lower,
and 802 upper-shoulder angular. Each alternative has its own Chinese authoring
prompt and complete numeric snapshot. Select by explicit ID; do not edit the
accepted baseline to switch styles.

The angular preset uses an opt-in monotone upper-coordinate warp. Skin, pocket,
ink rails and blink helpers share the same shoulder locations; native sampling
contains those exact stations, and the main upper ink rail uses one-sided slopes
there. Other rails keep their existing interpolation. Angularity defaults to zero;
legacy presets keep their prior contour behavior. Angularity currently requires
level corners and full lash following. More aggressive values are not implied safe.

The per-preset native script hashes identify the actual executed source. A base
commit alone is insufficient to reconstruct a candidate generated with working-tree
changes. Retained native model archives include the generated source and evidence.

All eight recorded candidates have 84 source-bound native captures each (672 total).
Generation and receipt-bound SDF completed; neutral, oblique, white and closed-eye
comparisons were inspected. See [native evidence](native-evidence.json) for analytic
orientation and sampled attachment measurements, including their scope limits.
The 802 orientation minimum is positive but small (0.00591148); this is a tested
first candidate, not evidence that stronger angular parameters or every face work.
Owner aesthetic acceptance is pending. Failed intermediate candidates are excluded.

## Reference-match correction

The owner rejected the eight-style study as insufficiently faithful to the researched
contours. Technical checks remain valid but are not shape-family acceptance. See
[reference review](reference-review.zh-CN.md). Candidate 803 is an independent
corner-taper experiment, not a replacement of 796 or a solution for all families.

The experimental [805 rounded upper/shallow lower candidate](rounded-arch-shallow-805.json)
uses actual upper/lower arch allocation rather than unequal base multipliers.
`aperture_upper_share=0` disables redistribution; active values .2–.8 partition
the sum of the two arch amplitudes and shift the iris center by the nominal
aperture-center change. This does not preserve opening height at every horizontal
position or all iris occlusion. The existing vertical-envelope checks remain active.
`lateral_guide_inward_power=0` keeps the prior guide selection; active values 1–4
select the endpoint-guided inward curve without changing local fit thresholds.
`upper_corner_softness` reduces the adapted upper exponent by up to 30%; default 0
preserves prior behavior. In 805 the upper exponent is .805, so its endpoint is
again round/vertical-tangent while the lower exponent stays 1. Do not apply 803's
finite-upper-endpoint claim to 805.

804 passed technical checks but was rejected visually for a triangular apex.
805 softens and widens the apex; both preserve shallow lower structure. Native
generation, receipt-bound SDF and 84 captures are recorded per candidate. 805's
continuous orientation minimum is positive but small (.00995316), and its 16
closest-parent bands at five blink weights reached 0.2248 mm; these do not prove
all-root continuity or absence of all collisions. Other reference families remain
unfinished, and technical success is not the owner's aesthetic acceptance.

## Reference-refined selection

The selector also accepts `almond-taper-experiment-803`,
`rounded-arch-shallow-805`, `asymmetric-angular-807`, and
`soft-wing-downturned-808`. Each is independently authored and verified; the old
796–802 snapshots remain available as initial-study records. Visually rejected
804 is deliberately not selectable. Use a fresh output prefix for every replay.

The selector packages with the currently checked-out code; recorded recipe commit
fields do not automatically restore historical implementations. A selected snapshot
is not a claim of byte-identical historical output. Native generation, SDF and
visual checks are required before adopting any replay. After self-review passes,
continue to the next workflow stage; per-iteration owner review is not a gate.

Recorded GLB, complete parameter files and every captured image for 803/805/807/808
were rechecked locally against their stored manifests. The checked metadata is in
[refined native evidence](refined-native-evidence.json). Repository tests verify
coverage and parameter/identity consistency; they do not contain or rerender the
large native asset archives. Each of the four IDs also successfully packaged through
the real CLI with a new replay prefix. Packaging alone is not a new native replay.

## Lash styling is a separate design decision

Do not assign the same dense lash ornament to every aperture. The retained round-open
and sharp-upturned variants can keep richer outer groups. Low-opening, downturned
and segmented contours benefit from a lighter band and fewer tips. This does not
change the independently authored aperture parameters.

`clean-downturned-810` keeps the true descending corner of 808, uses a .82 primary
band scale, and retains one short child blade per eye. Disabling layered lashes also
shortens the main tail and changes child positioning; it is not a pure deletion
operation. Its native component list, SDF and 84 captures were checked. Two remaining
child-root bands at five blink weights reached 0.0604 mm, below the unchanged 0.8 mm
limit. Front, oblique, white and closed-eye visual self-review passed.

809 isolates band thinning with all the richer components retained. It passed
technical and visual checks but the outer group still dominates; it is a controlled
comparison rather than the selected soft-family style.

`clean-angular-811` keeps 807's independent segmented aperture and simplifies only
the lash styling. The normal and closed contours passed visual self-review. Its
first screenshot set was rejected because interactive shortcuts contaminated the
camera and visibility state; geometry/SDF were unaffected. A fresh input-isolated
84-frame set passed actual yaw/pitch, orthographic size, head-only visibility,
expression and image/source hash checks. Do not use the rejected first set.

For new captures use the input-isolated
[eye-family probe](../../head-review/tests/eye_family_capture_probe.gd), which keeps
body/hair hidden and fixes orthographic size before every frame. Run
`verify_capture.py --workspace WORKSPACE --manifest MANIFEST.json
--expected-source res://assets/aster/SELECTED_PREFIX/aster_head.glb` afterwards.
The verifier rejects missing actual view metadata, perspective/hair/body changes,
actual render-mode or pose/zoom/expression mismatch, viewport changes, hidden faces,
unexpected or changed sources/images, PNG header/dimension mismatch, and incomplete
capture schedules. PNG header verification does not prove complete decoding or
nonblank meaningful content; visual review remains required. The legacy capture manifests are historical evidence, not passing inputs
for this stricter new verifier. It still does not judge beauty or prove collision
freedom.

## Current family choices

The catalog's `current_family_choices` identifies the working comparison set:
795 retained round-open, 803 balanced almond, 812 clean slender, 810 clean
downturned, 799 sharp-upturned, 800 shallow-upper/round-lower, 805 rounded-upper/
shallow-lower and 811 clean segmented. `baseline-795` denotes the retained baseline
asset and prompt; it is deliberately not an alternative selector ID. The other
seven choices resolve through `bundle.py --preset`. These are design starting
points, not a universal taxonomy or approval for arbitrary parameter combinations.

811 and 812 were generated through that explicit selector with fresh output
prefixes and complete snapshots, then received native SDF and visual review.
Both kept their original aperture families while simplifying the lash ornament.
812's sampled child roots reached 0.0621 mm; its continuous orientation minimum
remained positive at .0624564. The three simplified candidates are not applied
to the richer round-open and sharp-upturned styles.

## Fixed face-and-eye experiments

The explicit preset entry point also accepts `face-jaw-wide-817`,
`face-jaw-narrow-818`, `face-wide-819`, and `face-chin-narrow-820`.
Each freezes the full 810 eye configuration together with one tested silhouette
change. These are complete combinations, not independently interchangeable eye
and face menus. The original 795 prompt remains unchanged.

Loading checks the complete configuration digest, required width/visual evidence,
and 84 eye plus 18 mouth capture counts. The digest detects configuration drift;
it does not rerun native verification or certify a changed generator. Tests also
compare replayed final calibration with the recorded native calibration digest.
Every fresh native output still needs its own geometry, SDF, attachment and visual
checks. Alternate eye combinations, mixed axes and larger amplitudes are new
experiments. No user aesthetic acceptance or universal-face claim is implied.


### Native repeatability

The first full repeat (820 → 821) failed indexed topology equality. The inherited
posterior return created vertices by iterating a set of BMesh vertex objects.
Fresh semantic recipes now freeze original vertex indices and sort boundary
edges and source vertices before adding the return. The historical checkpoint
and accepted prompt files are unchanged; geometry formulas and safety checks
are unchanged. See `replay-821-rejected.json` for the failed check.

Two fresh generations (822 and 823) using `face-chin-narrow-820` then matched
all 10,980 indexed Face polygons and all 6,188 vertices in Basis, MouthOpen,
blink.L and blink.R exactly. UVs, mesh attributes, material slots, shape-key
order/relative bases, identity transforms and corner normals also matched.
This is a same-environment repeat of one fixed configuration, not proof for
all presets, platforms or material-node contents. Binary file hashes may differ
because artifacts carry their output paths and metadata.
