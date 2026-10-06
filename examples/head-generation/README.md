# Semantic face-generation calibration recipe

This is a recoverable development example, not an accepted character asset or a finished general-purpose face generator. It preserves the specification → explicit parameters → Beaver generation → native comparison loop used in the Aster face calibration experiment.

The executable modules and preset correspond to candidate 89. Python formatting was normalized with AST equivalence checked against the executed source. The portable bundler only packages source into a declared `blender.start` request; it does not launch Blender, call a model service, or create a coding task.

## Inputs and execution

The existing Beaver project must contain `project.godot`, `authoring/head_generation_guide.md`, and the frozen `assets/aster/head_recovery_49/` baseline with `aster_head_editable.blend`, `aster_head.glb`, `aster_definition.tres`, `hair_base.png`, and `hair_ilm.png`. These project assets are deliberately not published here. Restore the user's project checkpoint rather than substituting third-party geometry.

Run `python3 examples/head-generation/bundle.py --output /tmp/head-recipe.json` to package the request. Load that JSON into the existing authorized Beaver external-agent workbench, execute `blender.start`, and poll the returned job until terminal. Output paths are creation-only under `assets/aster/head_recovery_89/`; an existing output is a conflict, not permission to overwrite. For a new candidate, change the output prefix consistently in `bundle.py` and `pipeline.py` before packaging.

Use Beaver's `npr-character` workflow validation and its native MiDot review to inspect the produced definition. The companion `examples/head-review` scripts provide synchronized comparison, white and wire views, and the candidate-only `MouthOpen` slider. The normal state is mouth closed (value 0).

## Preserved behavior

Candidate81 adds a shared curved neutral seam for aperture, jaw classification and cavity normalization; explicit neutral half-gap and tapered upper/lower rim depth controls preserve a narrow rest mouth without the prior triangular opening regression. Four focused mouth tests join the eight existing recipe checks.


- Explicit normalized case proportions, sparse semantic cross-sections and eyelid/cavity/iris depth relationships. Generation does not read the visual reference asset or use a character name as an instruction.
- A closed chin return with quad underside bands, three side quads and one corner triangle per half; separate orientation checks prevent flipped panels.
- Hidden eye-pocket walls respect a sampled clearance against the evaluated facial shell. This is not a global collision certificate.
- Asymmetric pinnae with paired front/back quad bands and bounded evaluated-surface root fitting.
- Explicit short rising brow proportions and independently generated iris/lid/nose graphics.
- A quad skin control cage, bounded subdivision, expanding recessed eye pockets and separate concave iris surfaces.
- A real inward-facing oral cavity and thick tongue, exported neutral `MouthOpen` shape key and tangents.
- Face-only GLB replacement with frozen Body/Hair primitives and original binary prefix preserved.
- Deterministic source hashes and declared outputs through Beaver; no engine or pinned NPR package patch.

Some assembly, mouth and material coordinates remain specific to this restored project. They must be parameterized and tested before claiming a universal template. The current preset is a calibration case, not a universal beauty rule.

## Evidence and limits

Historical baseline 75 completed through Beaver on 2026-10-06 with exit code 0. Its GLB SHA-256 is `e9b6c82a70b57a9f3f4ceebe481811407196f6dae1f3d5ebb7e70d5ba1615847`. Native front/side/white45 review on 2026-10-06 confirmed retained chin closure and recessed eye placement. The pinna now has an asymmetric semantic outline, concha/helix bands and a conforming closed back shell; a single nonplanar rear cap was rejected after failed volume checks. Sampled evaluated-shell fitting removes the prior ear-root gap without the earlier cheek wing. This is overlapping attachment, not a welded mesh or a global collision guarantee. Sparse brow proportions and independent procedural iris/lid/nose pigments are explicit case parameters. Native MouthOpen0/0.5/1 regression retained the neutral slit and the dark cavity/tongue without white seams. Ear lower-root shape, lip volume and overall reference resemblance still need work.

Ear, eye, nose, lip and overall reference resemblance remain under visual iteration. This source checkpoint is not visual approval, full wardrobe acceptance, a production deployment, or a release. The screenshots and complete editable project are retained separately in the user's project backup.

Local checks: `python3 -m unittest discover -s examples/head-generation -p 'test_*.py'`; compile the module sources without importing Blender-only modules. Repository CI remains responsible for the current aggregate gates.


## Reconstructed81 checkpoint
After workspace reset this source restores recorded lip-volume/seam decisions from75. Pure math and packaging tests do not establish native visual acceptance. Do not claim original76–80 files recovered. Candidate81 has now run through Beaver; see review-results.md for the limited observed results and remaining issues.


## Candidate83 focused update
Broad planar ocular rear caps with geometric flat corner normals remove the previously observed pinching/bands. Pinna geometry is simplified and detail shifted to original procedural pigment. See review-results.md for actual evidence and open limitations; no full-face acceptance.

## Candidate 85 update
Real semantic edge-crease profiles and independently parameterized nasal basal/tip falloff are retained. Nineteen focused tests and native regression views are recorded in review-results.md. Original reference crease weights are unknown; this is an authored calibration, not recovery of them.

## Candidate 86 oral volume
Explicit normalized oral rings retain the neutral lip attachment while enlarging hidden rear volume. Native rear/front and mouth regression checks are recorded separately;20 local tests pass.

## Candidate 88 paired eye alignment
Raised, slightly shorter independent iris and coordinated lower eyelid preserve recess while avoiding the lower white crescent observed in the isolated87 trial.21 focused tests pass; see review-results.md for limited visual evidence and remaining work.

## Candidate 89 uniform iris topology
Each iris now uses 81 vertices and 64 quads without compressed pole rows.22 focused tests and limited native views are recorded; user feedback on ears, upper boundary, profiles, brows/lips and chin motion remains open.
