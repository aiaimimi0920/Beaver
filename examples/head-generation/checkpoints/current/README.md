# Ear-under connection checkpoint518

Source-only partial recovery checkpoint,2026-10-12. Reference-equivalent appearance and owner acceptance remain pending. No reference mesh or texture included. Eyelashes remain frozen.

## Reproduction
Requires the owner's existing Beaver project, private assets/aster/head_recovery_49/aster_head_editable.blend, authoring/head_generation_guide.md and NPR runtime dependencies. Not standalone. Package with python3 bundle.py --output request.json, load through native Beaver blender.start, then native NPR validation and preview. Do not execute pipeline.py directly. Editable blend is intermediate; runtime_morph blend and GLB include final post-join changes.

## What changed
The owner's new side-view report led to eight matched yaw comparisons,0,+/-45,+/-90,+/-135,180.516 had an overly straight ear-to-jaw posterior contour: its middle/lower edge projected up to8px forward of reference while the ear-root end was already too far back.517's uniform amplitude increase improved the middle but regressed the upper attachment, so it was not published.
518 redistributes posterior curvature: compact height support .025/.11/.27 and .014m amplitude replaces516 .035/.11/.38 and .006m. The lower arc fills out while the broad upper tail is reduced. Existing transverse return widths,pinna shape and eyelashes are preserved.

## Verified evidence
281 pure tests passed in20.589s. Native job480fc964-dc44-46c4-83f7-d931edfe1cde terminal poll2026-10-12T16:48:07.323Z,exit0,51241ms;32 output hashes verified. NPR2026-10-12T16:48:41.873Z ok:true,errors:[]. Actual516 to518:469 unique Face positions changed,maximum nearest displacement6.0568mm;5862 unique vertices in each. Positions outside ear/posterior support and body/hair remain unchanged. Native checks reject inverted/collapsed faces and shared-root failures. Tests are not zero-warning or aesthetic acceptance.

## Visual review and limits
Eight matched render angles captured before and after. Also inspected side white and wire. At matched right90 rows680..740, posterior silhouette errors reduced from3,5,8,8,8,6,5px to1,1,2,2,0,-1,-1px. These are image-space samples only. Ear-root/lobe vicinity still differs (row640 roughly10px;row6504px), and fine surface/topology appearance remains subject to review. Front and posterior return remain visually stable in examined views. This is an incremental repair and backup,not final product acceptance,merge or deployment.
