# Lower contour candidate 739 — 2026-10-17

## Native evidence

- Baseline 736 and candidates 737/739 were compared using the same smooth_proxy SDF policy, orthographic camera, light and pose sequence.
- 737 reduces lower_arch to 0.70. 739 additionally uses lower_roundness 1.25.
- 738 (lower_roundness 2.0) was rejected by the native continuous-pose triangle orientation guard. No output from that failed candidate is accepted.
- 739 generation job: 6988b7b3-ef72-4fa2-9ae4-bf593563c2be, succeeded.
- 739 GLB SHA-256: c597b52b9c3a23c999039d907c4f7d30cd67cedacd466a2bb34d140f07865e68.
- 739 smooth_proxy SDF job: 502a1b50-1dd0-4764-b633-963896b9ae25, succeeded.
- 739 native capture manifest: blink-1792245814-40127, 69 captures. All image hashes and requested/read-back morph weights verified; source-before and source-after matched.
- Matched front and +/-45-degree render, white-model and closed-eye boards were visually reviewed. Ordinary interactive preview was restored and opened with the 739 definition afterward.
- The shallower lower aperture reduces the round opening appearance. This is an incremental retained candidate, not final user aesthetic acceptance or a proof of universal character attractiveness.

## Boundaries

- Generation orientation check minimum ratio: 0.0132135041418463 across 10,884 triangles; worst pose blink=1, mouth=0. This does not certify all intersections or arbitrary parameter combinations.
- Exposed lower_roundness range is provisionally capped at 1.25. The previous 2.0 experimental candidate must not be described as supported.
- Default 1.0 preserves legacy pocket classification. Non-default values opt into the lower inverse-power classification; this is a discrete compatibility change, tested on both sides of its threshold.
- The white-model inspection still exposes inherited surface/shading irregularities. Passing the native pose guard is not final product acceptance.
- An initial 737 bake used mesh_normals; that unmatched lighting comparison was discarded and 737 was rebaked with smooth_proxy before comparison.
- No hair/body work, deployment or release is included.

## Verification

Run semantic_eyes and facial_surface unittest discovery, the direct-display Node tests, syntax checks and diff whitespace checks. Native render evidence above supplements CPU tests rather than being inferred from them. The renderer change limits only presentation copies; authoritative tool requests, receipts and context remain intact. Long diagnostics preserve their head and tail.
