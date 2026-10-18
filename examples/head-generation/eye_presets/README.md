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
  oblique and closed-eye comparisons were inspected. Owner visual acceptance remains
  pending; this is a subtle almond candidate, not a universal aesthetic acceptance.

`bundle.py --preset almond-balanced-796 --prefix assets/aster/general_face_eyes_796/
--workspace WORKSPACE --output REQUEST.json` packages a request only. Execute it through
Beaver's native Blender tool, then pass the successful generation receipt and the
preset's complete `eye_parameters` to `semantic_eyes/sdf_bundle.py` with its declared
shading policy. The original authoring inputs are still required. Use the normal
NPRCharacterFrame/MiDot validation and same-camera neutral, oblique, white-model,
and blink captures. CPU tests do not establish visual quality.

The selector checks hashes of all three preserved 795 files and refuses the baseline
model directory. Its request-output path must also be outside retained recipe directories. Reusing an existing non-baseline prefix still follows the existing
packager's expected-hash overwrite checks. Future presets need their own authored
configuration, prompt and verification; this entry does not implement eight eye types.

The `recipe_commit` records the geometry recipe used for the recorded candidate;
it does not silently check out that commit. Use that code revision for historical
reproduction. If the underlying recipe changes, regenerate and verify a new candidate.

Tests: `python -m unittest discover -s examples/head-generation/eye_presets -p 'test_*.py'`
