# Upper-eye iteration evidence — 2026-10-17

This is an experimental shape-control checkpoint, not final aesthetic acceptance.
All native candidates use the same smooth_proxy SDF policy and face-only review.
No source mesh is restored from an accepted final model.

## Controlled candidates

- 740 changes only short_upper_tuft to 1 from 739. Generation 22ea9cea-7404-4a4a-9f36-aafacea46178 and SDF 0cb70479-7c1d-4835-a104-c592658662f6 succeeded. GLB SHA-256 a1101a6eb0a0dbe3cb0be5abb7bfab0cd8c7b946bceaad11070b938c294ada60. 69 captures in blink-1792247816-93387 verified; frontal/oblique partial and full blink boards inspected. The small outer accent improves detail but does not solve the symmetric aperture.
- 741 changed upper_roundness to 1.25 on 740. Native job 078a654b-fb46-41e3-8ba6-9e40a9535215 was rejected for a folded posed triangle. No accepted outputs; the guard was not relaxed.
- 742 uses the milder upper_roundness 1.45. Generation a83e6199-6983-491a-8a88-8b069b239444 and SDF 72c356a7-d5a3-4369-9aef-93faf9ad608b succeeded. GLB 52de0a944ad076f78e2349105f4dc0f6db3e0c40bd8ffefad468e9a2cd6e504c. 69 captures in blink-1792248760-34983 verified and reviewed. This is a modest retained intermediate, not a final face approval.
- 743 adds upper_asymmetry 0.05 on 742, keeping width, height maxima, centers, lower contour and iris shape/position parameters fixed. Ocular support fitting can still change depth/tilt. Generation 4b0cd3fb-50d6-4b3e-a4ef-938a05dcd1dd and SDF cdc810b8-49f6-4ca0-9e84-066f7b82f8c8 succeeded. GLB f42d48a2bf06fe50325d1cc1fa406282960e7e8a236b19a639f34ca24ba97941. 69 captures in blink-1792249897-32812 verified. Matched front/oblique render, white-model and closed-eye boards reviewed; visual effect is subtle, further comparison remains open.

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

## Expanded upper-warp range

The experimental range is now [-0.25, 0.25]. Its warp derivative is bounded below
by approximately 0.570675; the inherited outer inverse branch remains monotone
for level corners. Extrema are covered in contour, interval, inverse and package
source tests. This is a mathematical control-domain check, not a mesh safety proof.

A pure-CPU diagnostic using candidate 746's derived face source found an inherited
upper/lower support-field branch difference at the eye-corner baseline. On 2,001
samples across t in [-1.16, 1.16], baseline-side depth differences ranged roughly
0 to +0.1873 mm at a=0 and -0.4850 to +0.6037 mm at a=0.25. The maxima lie inside
the eye aperture, not on exposed facial skin; the iris and pocket use separate
depth functions. Outside |t| >= 1 the warp adds no difference. These are sampled
values, not proven extrema or evidence of a visible defect. Interval enclosure
tests must not be described as continuity tests. Native mesh/white-model/pose
review remains required; no speculative support-field repair was included.

## Native range-extrema follow-up

744 (+0.10) and 745 (-0.10) both completed native generation, matching SDF and
69 verified captures. The outward-weighted alternative 745 looked more lifted;
744 was weakly preferred, with only subtle full-face improvement.

746 (+0.25) generation c8cb7bc1-2dee-4bd6-9e4e-97e06837a9c5 and SDF
3beed92d-e90c-408d-9ee2-f7a64bc06a4e succeeded. GLB SHA-256:
cf044b73b519b2ac2e08dc9362da8f491691b0583290b8aada397898e5cd7348.
All 69 captures in blink-1792251846-16213 passed image hash, pose read-back and
source consistency checks. Front, oblique, profile, elevated/lowered views, white
model and partial/full blink boards were inspected. The upper contour reads less
centrally rounded; this is still an aesthetic intermediate, not user approval.
Continuous orientation minimum: 0.010511406892573683 over 10,980 triangles.

Read-only inspection of the actual 746 mesh found no skin vertices in the central
baseline region with the largest analytic support-field jump. Eight skin triangles
cross the baseline close to the corners (|t| about 0.963 to 1); the sampled added
analytic difference there was below 0.7 micrometers. This is not a mesh crack
measurement, nor an intersection proof. No support repair was justified by this
diagnostic. The white-model facial banding remains visible in the baseline too;
no claim is made that the full facial surface has passed final aesthetic review.

After the range expansion and extrema packaging tests, 104 semantic-eye and
30 facial-surface tests passed again. Negative -0.25 has CPU/package coverage,
not an additional native aesthetic acceptance run.

The retained example selects candidate 744 (+0.10), not the newest candidate.
Independent visual comparison preferred its softer outer upper-lid transition.
746 makes asymmetry clearer, but its longer flattened outer lid reads somewhat
heavier/drowsier and does not resolve the narrow far-eye appearance in obliques.
It remains a useful range-extrema experiment rather than the preferred preset.
