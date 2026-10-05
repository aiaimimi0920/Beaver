# Semantic face-generation calibration recipe

This is a recoverable development example, not an accepted character asset or a finished general-purpose face generator. It preserves the specification → explicit parameters → Beaver generation → native comparison loop used in the Aster face calibration experiment.

The executable modules and preset correspond to candidate 64. Python formatting was normalized with AST equivalence checked against the executed source. The portable bundler only packages source into a declared `blender.start` request; it does not launch Blender, call a model service, or create a coding task.

## Inputs and execution

The existing Beaver project must contain `project.godot`, `authoring/head_generation_guide.md`, and the frozen `assets/aster/head_recovery_49/` baseline with `aster_head_editable.blend`, `aster_head.glb`, `aster_definition.tres`, `hair_base.png`, and `hair_ilm.png`. These project assets are deliberately not published here. Restore the user's project checkpoint rather than substituting third-party geometry.

Run `python3 examples/head-generation/bundle.py --output /tmp/head-recipe.json` to package the request. Load that JSON into the existing authorized Beaver external-agent workbench, execute `blender.start`, and poll the returned job until terminal. Output paths are creation-only under `assets/aster/head_recovery_64/`; an existing output is a conflict, not permission to overwrite. For a new candidate, change the output prefix consistently in `bundle.py` and `pipeline.py` before packaging.

Use Beaver's `npr-character` workflow validation and its native MiDot review to inspect the produced definition. The companion `examples/head-review` scripts provide synchronized comparison, white and wire views, and the candidate-only `MouthOpen` slider. The normal state is mouth closed (value 0).

## Preserved behavior

- Explicit normalized case proportions, sparse semantic cross-sections and eyelid/cavity/iris depth relationships. Generation does not read the visual reference asset or use a character name as an instruction.
- A quad skin control cage, bounded subdivision, expanding recessed eye pockets and separate concave iris surfaces.
- A real inward-facing oral cavity and thick tongue, exported neutral `MouthOpen` shape key and tangents.
- Face-only GLB replacement with frozen Body/Hair primitives and original binary prefix preserved.
- Deterministic source hashes and declared outputs through Beaver; no engine or pinned NPR package patch.

Some assembly, mouth and material coordinates remain specific to this restored project. They must be parameterized and tested before claiming a universal template. The current preset is a calibration case, not a universal beauty rule.

## Evidence and limits

Candidate 64 completed through Beaver on 2026-10-05 with exit code 0. Its GLB SHA-256 is `bd531ce8bc790d75dc7cfd62e6c09d4010594b29007666b5720de36a85e5865a`. Native comparison verified that normalized cross-section lofting removed the previous edge waves and patchy shading. Earlier candidate 62 also exercised the subdivided mesh at full mouth opening, showing the dark cavity and tongue without the earlier white wall seams.

Eye, nose, lip and overall reference resemblance remain under visual iteration. This source checkpoint is not visual approval, full wardrobe acceptance, a production deployment, or a release. The screenshots and complete editable project are retained separately in the user's project backup.

Local checks: `python3 -m unittest discover -s examples/head-generation -p 'test_*.py'`; compile the module sources without importing Blender-only modules. Repository CI remains responsible for the current aggregate gates.
