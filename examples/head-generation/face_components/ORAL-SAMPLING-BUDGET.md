# Optional oral sampling and skin-normal isolation

This is a bounded source-generation increment, not final facial visual acceptance.
Default nine-column/detail-off generation and all eye-ring sampling are unchanged.
For thirteen-column skin with detail budgeting enabled, the oral patch now uses
radial fractions [0, .50, .75] before appending its unchanged outer boundary.
The existing depth formula and opening definition remain in use. No character
name, exported vertex index or post-export decimation controls this change.

Faye candidate 95 has 4,264 actual whole-Face triangles, 60 more than candidate 90
and 54 more than the 4,210-triangle reference. Within the same mouth spatial ROI,
P90 dihedral angle fell from 34.251 to 18.884 degrees and the maximum from 81.058
to 51.231 degrees. This does not prove a natural-looking mouth or absence of
self-intersections. The earlier uniform alternate-diagonal trial worsened quality;
the subsequent dominance selector reproduced the existing BEAUTY triangulation.
Neither diagonal experiment is retained here.

Candidate 96 additionally excludes non-skin regions from the skin smoothing
accumulator in that same optional profile. Per-region normals and the explicit
ear-root blend remain. All triangles, three UV sets, both colours and six morph
POSITION streams exactly match candidate 95. 106 corner normals/tangents changed
at 36 positions: 30 around the mouth and 6 near the ear/ear underside. It would be
incorrect to describe this as a mouth-only change. Normals remain finite/unit.

## Actual bounded evidence

- 95 and 96: strict native generation, independent source packaging, fresh SDF,
  and 36 real NPR/white/wire/expression captures each. Image hashes and renderer,
  model and texture bindings stayed stable during capture. Normal entry restored.
- 96 GLB SHA-256:
  0f2e674ee17fe0168f303ca11ec655f9fe4d1a09c2cf3b10a2b12d64acab1466
- The actual views show a smaller/blunter outer-mouth triangular shading edge
  and softer 45-degree transition relative to 90. Short upper-lip ripples remain,
  and upper-lip projection is slightly reduced in profile. The normal-isolation
  step itself is visually near-neutral; no new visible ear-seam regression was
  found in the reviewed views. These are partial gains, not full acceptance.
- Public composition cross-check 97 (blunt chin, elongated nose, lip-band mouth)
  strictly generated 4,264 triangles. SHA:
  bad8ebeb7a2117ed21ec652d251c9a34d022300138ee99b69bcb658ae62d17b0
- Public composition cross-check 98 (compact-rounded face, balanced nose, line
  mouth) strictly generated 4,264 triangles. SHA:
  6381837f8910503d34953499ca109e02d1b5e370c94b2b10bc862767258e6729
- Those two cross-checks are generation checks only, not separate SDF/render
  acceptance, all-template coverage, collision clearance or speech validation.
- Local focused suites: semantic 226, source-layout 29, components 122, facial
  surface 117 (494 total). Three new tests exercise profile boundaries and the
  actual generated normal-accumulation branch; prior source tests check helper
  embedding. The normalized style omits default 13, so omitted/explicit cases
  are both covered.

Forehead oblique folds, cheek bands, jaw transitions and some oral ripples remain.
Keep the PR draft and preserve the prior comparison baseline. CI success and
angle statistics are not a substitute for visual/product acceptance.
