# Candidate 123 progress checkpoint, 2026-10-06

In progress, not final aesthetic acceptance. Accepted center division and the 120 baseline remain preserved.

## Verified changes
- 121 removed six inactive historical calibration controls. The first attempt incorrectly removed two conditional lip keys and failed before output; both were restored. The contract test now follows the conditional assignment used by C[rim_key] and checks missing as well as unused fields.
- Fixed 121 succeeded. Its GLB and face base/ILM/map textures are byte-identical to120, establishing that the six removed controls did not affect that generated result.
- 122 exposed seven original ear pigment placement/size controls; each has a tested nonzero effect. Actual side review still showed a compressed inner arch, so this was not accepted as a visual fix.
- 123 projects the original front ear UVs from the whole membrane side plane, retaining a plain reverse skin. In the actual side render the upper arch is now visibly spread across the shallow ear instead of a thin stripe at its root. Shape and exact concha pattern still differ from the reference.

## Evidence
- 45 focused Python tests pass; all13 generated source/config/document hashes match the declared input manifest.
- 123 generation succeeded: job29777b75-0aaa-43bc-894b-5125b149a872, request667fc090-7955-4b2f-ac91-1f6c2d486521.
- Native NPR validation9752d1ad-b4de-421f-be07-acab988ea151 passed at11:51 UTC (ok=true, errors=[], engineErrors=false).
- GLB SHA-25660165612dd40c07ee8bf540ad6e5066462de133e8f48da38c6cef59f9f0f9a88. Face texture SHA-256cbed4bbb0f6f138898fc92475ca90ca9cc439f2e19d3fc700025e9d2921e76bb equals122 intentionally.
- Read-only GLB comparison confirms exact equality of unique positions, position/normal pairs and oriented geometric triangles between122 and123. Export vertices increase4960→4970 due to UV attribute seams, not extra geometric points.
- Frozen Body/Hair and runtime/package pins remain unchanged; no reference meshes/UVs/pixels are loaded by the generator.

## Open gates
- Latest full project aggregate remains120: 7 tests / 57 assertions passed. This does not validate every future edit or establish full wardrobe acceptance.
- Ear attachment height/shape, concha detail and green-marked cheek/jaw transitions remain under visual calibration. Only the center shading division has explicit user approval.
- Existing export/UID warnings remain. Source120 GitHub workflow37454972931 failed before runner allocation; no GitHub CI pass, external review, merge or release is claimed.
- Models through120 are durably backed up;121–123 generated outputs and the failed121 diagnostic are preserved locally pending the next backup.
