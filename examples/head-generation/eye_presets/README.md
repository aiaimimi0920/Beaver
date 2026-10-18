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
configuration, prompt and verification. The catalog now provides seven explicit alternatives
to the retained 795 baseline.

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
