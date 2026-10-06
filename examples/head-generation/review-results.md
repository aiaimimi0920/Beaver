# Candidate119 progress checkpoint, 2026-10-06

Status: in progress; not final aesthetic acceptance. The user accepted only the center facial shading division seen in108. Preserve that behavior.

## Verified
- 41 pure tests passed locally. Changed runtime Python modules remain below500 effective lines (pipeline322, face235, ears158, textures190).
- All13 declared source/config/document hashes match the actual119 generation input manifest.
- Beaver blender.start/poll succeeded: job85b59876-189e-4b18-8243-fedc7f5dd844; request1eff4498-4d10-4626-93d9-c5417ab7264d.
- Native NPR validation passed at10:52 UTC: request3a780e44-a599-4462-a8bd- 419650b55273, ok=true, errors=[], engineErrors=false.
- 119 GLB SHA-256 e7ca57b9c8c7bc4f4c70faf5bfdbc3526fdc6d182f49127eb14ec74455f11f7f. Geometry equals118 intentionally;119 changes original external face texture only.
- 119 face_base.png SHA-256 f93739316ff11d18105903ec81a3f7df853b08eaa91e608419961a683b4c74e0.
- The118/119 lower-ear silhouette is visibly rounder than117 in actual synchronized side rendering.119 no longer draws the original full-height ear pigment stripe to the lower lobe. The reference still has a clearer inner arch and concha: this is unresolved.
- 111's removed lower-ear black wedge remains absent in the inspected side view. Root connectivity and frozen Body/Hair preservation checks remain enabled. No reference mesh/UV/pixels are loaded by the generator.

## Boundaries
- The project aggregate still targets102; no aggregate pass is claimed for119.
- Ear shape and internal shading, eye/lip differences and user-marked cheek/jaw transitions remain under review. A pure test pass is not an aesthetic approval.
- Optional Draco exporter/vertex-color and inherited UID warnings are not claimed absent.
- Previous exact published111 CI jobs failed before allocating a runner (runner_id0, steps[]); cause unresolved.119 has not yet been submitted to CI at the time of this source note.
- PR remains draft and unmerged; no external review approval, production deployment or release is implied.
