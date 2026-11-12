# Shared game-face source budget (work in progress)

The requested target is approximately 4,000 triangles for the entire generated
Face mesh, allowing a small excess near the 4,210-triangle reference. Count all
facial components, including lashes, eye pockets, iris, brows, ears and oral
geometry. Body and hair remain frozen and are not part of this face-only count.

The public composition route now selects GAME_FACE_SAMPLING. This creates a
coarse source shell without uniform subdivision, samples authored eye ribbons
more sparsely, and uses a quality-selected three-sector lower ear fan followed
by a conforming transition to the ordinary membrane rows. No exported mesh is
decimated. Sampling overrides remain explicit; sampling_first_pass=0 selects
the legacy sampling defaults. Non-researched brows retain their compatible
legacy brow sampler instead of receiving an unsupported research-only control.

Lateral eyeliner sections retain an explicit integer vertex attribute through
assembly. Blink displacement is averaged within each authored section and
applied as a rigid sectional translation. This preserves its mean movement
rather than reducing the overall blink amplitude to hide inverted triangles.

## Native evidence, 2026-10-24

- Previous retained density specimen: 8,056 triangles.
- Trial 49: 4,180; ear quality and closed-blink orientation failed. It is an
  explicitly diagnostic export, not an accepted model.
- Trial 50: 4,180; section-coherent blink passed the continuous quadratic
  orientation check over blink and MouthOpen in [0,1].
- Trial 51: 4,164; ear fan quality also passed. No ear-root reflow was necessary.
- Trial 52: strict-mode replay, 4,164; GLB SHA256
  60c214cae21866f2d1ccd1da512d98ad28e46d8e2b1fedd35019da489ce44658.
- Public-default blunt-chin/lip-band/elongated-nose trial 55: 4,188; strict native
  generation, ear quality and analytic blink/MouthOpen orientation passed.
  GLB SHA256 f4a9ee45ea14abbb1724cead420d59ceb4bb2ad811cedc3a332870559c360fe8.

- Public-default compact-rounded/line-mouth/balanced-nose trial56: 4,164;
  strict native generation, both ears and analytic blink/MouthOpen orientation
  passed. GLB SHA256
  9e23a683707b49fff6fd2cc5e518e64377571830a2bb2c973e3adff07434951c.
- Optional-fairing trial57: 4,164, strict native generation and SDF passed;
  36 actual NPR/white/wire captures verified against source and texture hashes.
  GLB SHA256 0dab5e1bd56e9f44f3ce41db5f79b7735f74f6550e2dd6665bc1c8e0360679f5.
  Review confirms the mirror-seam slit is removed, with modest jaw/cheek
  improvement. A shallow continuous top-center dip, faceting and angular ears
  remain; this is not restoration of all previous high-density quality.

These are generation/geometry checks, not final product acceptance. The default
recipe is composition-dependent; inspect the actual exported count for each
composition. There is no claim that every supported combination has been run.

## Experimental controls and outstanding acceptance

sampling_budget_trial defaults to zero. Only an explicit diagnostic request can
retain an artifact with recorded ear/pose failures; those failures remain failed
in its report and visual approval remains false. Normal generation raises them.

sampling_surface_fairing also defaults to zero. The optional bounded source-cage
placement trial 54 produced 4,172 triangles and passed numerical checks, but was
rejected visually for a top-center mirror seam slit. The mirror-plane constraint is verified in trial57; do not treat rejected
trial54 as accepted. Fairing remains optional rather than a new default.

Actual trial-49 NPR/white/wire capture inspection found recognizable face and
mouth shapes but increased silhouette faceting, angular ear rims and cheek
shading ripples. Those issues, additional combinations, final repaired-render
comparison and runtime acceptance are outstanding. The mouth-shape check samples
81 combinations; it is not a continuous all-shapes, self-collision, speech or
performance proof. CI success does not establish visual or deployment acceptance.
