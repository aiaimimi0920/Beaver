# Candidate 120 progress checkpoint, 2026-10-06

Status: in progress, no final aesthetic acceptance. The accepted center facial shading division from 108 remains preserved.

## Current results
- 41 pure tests passed; all 13 generated source/config/document hashes match the actual input manifest.
- Beaver generation succeeded, job 344cfe0b-6cc1-4b81-a33e-7b17497d5f6e, request 02148d5c-1479-4967-bd09-e115f45b727c.
- Native NPR validation passed at 11:00 UTC. Actual front/render and oblique white comparison were inspected.
- Aggregate validation 20e6b01d-faa6-410b-8bb0-7cc551ca3b92 completed at 11:11:20 UTC, verdict autoPassed, exit 0: 7 tests and 57 assertions passed. The head-review integration explicitly loaded candidate 120. This is not full wardrobe/product acceptance; the negative full-wardrobe gate remains intact.
- Current GLB SHA-256: 2c89b968eccb160b74e9bab326083d883720bd5ed77bac6280cc66c4c75bc0b5.
- Rounded ear lobe and shorter original ear pigment from 118/119 retained. Lower-cheek width uses a few rounded semantic controls to reduce pinching. Ear inner arch and marked cheek/jaw transitions still require visual refinement.
- Both ear roots retain 12 shared manifold edges with exactly two adjacent faces. Body/Hair binary prefix is preserved.
- Morph audit: front chin-tip 382 samples have zero movement. Lower skin-chin 780 samples have maximum downward movement 0.730 mm. Broader coordinate-only sets include oral structures and must not be called chin skin.

## Remaining limits
- Existing UID fallback and exporter warnings remain; passing tests do not mean zero warnings.
- Previous source 119 CI run 37452858541 failed before runner allocation (job 112233299304, runner_id 0, no steps). No current GitHub CI pass, review, merge or production release is claimed.
- Library version 37 preserves models through 120, but was created before updating the aggregate test target. Current tests and aggregate evidence must accompany the next checkpoint.
- A subsequent parameter-contract cleanup is staged separately, not part of this generated 120 checkpoint.
