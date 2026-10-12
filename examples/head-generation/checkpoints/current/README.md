# Ear-under connection checkpoint516

Source-only partial recovery checkpoint,2026-10-12. Reference-equivalent appearance and owner acceptance remain pending. No reference mesh or texture included. Eyelashes remain frozen.

## Reproduction
Requires the owner's existing Beaver project, private assets/aster/head_recovery_49/aster_head_editable.blend, authoring/head_generation_guide.md and NPR runtime dependencies. Not standalone. Package with python3 bundle.py --output request.json, load through native Beaver blender.start, then native NPR validation and preview. Do not execute pipeline.py directly. Editable blend is intermediate; runtime_morph blend and GLB include final post-join changes.

## What changed
Retain515's connected posterior return and bounded3mm ear-under transition.516 changes only ear_lower_arc_power .30 to .35 to slightly reduce excessive lower-lobe fullness. Shared roots and return construction remain connected; eyelashes and other features are frozen.

## Verified evidence
281 pure tests passed in20.284s. Native job7164b0f6-1787-47d9-b2e8-91abafb6adbd completed2026-10-12T15:42:06.684Z,exit0,50118ms;32 output hashes verified. NPR2026-10-12T15:42:51.370Z ok:true,errors:[]. Actual515 to516:336 unique Face positions changed,maximum nearest displacement1.1512mm;5862 unique vertices in each. Positions outside ear/posterior support and body/hair remain unchanged. Tests do not imply zero warnings or aesthetic acceptance.

## Visual review and limits
Inspected actual front,right90,rear180 render and150 white/wire. The rear lower-lobe outer discrepancy at sample row640 decreased from3px to1px, while its colored width remains3px greater than reference; other sampled posterior-band widths are generally close but not identical. These are matched-camera image observations,not physical measurement. Ear-root fine shape and shading remain subject to owner review. Incremental backup,not final acceptance,merge or deployment.
