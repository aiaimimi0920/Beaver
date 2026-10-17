# Upper-eye iteration evidence — 2026-10-17

This is an experimental shape-control checkpoint, not final aesthetic acceptance.
All native candidates use the same smooth_proxy SDF policy and face-only review.
No source mesh is restored from an accepted final model.

## Controlled candidates

- 740 changes only short_upper_tuft to 1 from 739. Generation 22ea9cea-7404-4a4a-9f36-aafacea46178 and SDF 0cb70479-7c1d-4835-a104-c592658662f6 succeeded. GLB SHA-256 a1101a6eb0a0dbe3cb0be5abb7bfab0cd8c7b946bceaad11070b938c294ada60. 69 captures in blink-1792247816-93387 verified; frontal/oblique partial and full blink boards inspected. The small outer accent improves detail but does not solve the symmetric aperture.
- 741 changed upper_roundness to 1.25 on 740. Native job 078a654b-fb46-41e3-8ba6-9e40a9535215 was rejected for a folded posed triangle. No accepted outputs; the guard was not relaxed.
- 742 uses the milder upper_roundness 1.45. Generation a83e6199-6983-491a-8a88-8b069b239444 and SDF 72c356a7-d5a3-4369-9aef-93faf9ad608b succeeded. GLB 52de0a944ad076f78e2349105f4dc0f6db3e0c40bd8ffefad468e9a2cd6e504c. 69 captures in blink-1792248760-34983 verified and reviewed. This is a modest retained intermediate, not a final face approval.
- 743 adds upper_asymmetry 0.05 on 742, keeping width, height maxima, centers, lower contour and iris fixed. Generation 4b0cd3fb-50d6-4b3e-a4ef-938a05dcd1dd and SDF cdc810b8-49f6-4ca0-9e84-066f7b82f8c8 succeeded. GLB f42d48a2bf06fe50325d1cc1fa406282960e7e8a236b19a639f34ca24ba97941. 69 captures in blink-1792249897-32812 verified. Matched front/oblique render, white-model and closed-eye boards reviewed; visual effect is subtle, further comparison remains open.

Every capture verification checks image SHA-256, requested/read-back morph weights,
and identical source-before/source-after hashes. These checks do not independently
certify attractiveness, all collisions, or all possible control combinations.
743's continuous orientation minimum is 0.010628929167215806 across 10,980 triangles,
with worst blink 1 and mouth 0. The guard still requires strictly greater than 1e-5.

## CPU and packaging evidence

104 semantic-eye and 30 facial-surface tests passed before native 743 review.
Coverage includes zero identity, monotonic warp, analytic derivative, actual contour
and blink agreement, outer inverse round trips, native depth versus interval field,
facial preflight/source agreement, retained helper provenance and 64-output packaging.
The helper parameter range is experimental. A successful sample does not validate
the full Cartesian product of parameters. Existing geometric envelope, reviewed
surface support budget, clearance and attachment guards remain in force.

Pose-failure diagnostics now include triangle/vertex IDs, orientation ratio and
minimizing blink/mouth weights (formatted to nine significant digits). This only
improves rejected-job diagnostics; the acceptance condition and morphs are unchanged.

## Visual research boundaries

Five game-model references were compared: Silver Wolf, Jean, Belle, Rover and Selena.
They support varied inner/outer contour rhythm and dominant upper-lid line weight;
they do not establish one universally correct eye height, peak direction or face
ratio. Their cameras, expressions and hair occlusion differ, so proportions were
not copied as anatomical measurements. Supplemental official/promotional 2D art and
anime designs were kept separate from 3D model evidence. No third-party images are
included in this repository change.
