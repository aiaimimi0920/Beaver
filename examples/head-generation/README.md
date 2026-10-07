# Semantic head calibration candidate 177

Original procedural recipe for the existing Beaver workflow. This is a runnable head-review candidate; owner aesthetic acceptance and whole-character acceptance are separate gates.

The current batch starts from the owner’s candidate 162 feedback. It preserves the accepted eye aperture, iris, pupil, catchlight and independent brow, the narrow neutral lip seam, small chin-follow motion and frozen body/hair.

## Scoped changes
- Corrected the directed lower-ear bridge edges that produced the black triangle.
- Smoothed the posterior chin transition and eliminated the small side-profile knee.
- Refined the three requested eye-adjacent accents and original ear pigment/return.
- Calibrated lateral lip relief and rolling curvature using real 10-degree paired reviews.
- Fixed mouth-corner patch correspondence; deeper oral rings retain a regular ellipse.
- Separated lip-detail geometry from broad facial SDF shading to remove local shadow spots.
- Added linked near framing, Shift-drag panning, framing reset and exact 10-degree steps to the example viewer.

86 pure tests pass. Generation and native validation passed. Actual 19-angle neutral comparisons, white/wire views, near framing, light sweeps and mouth motion were reviewed. Current whole-project aggregate status is recorded separately in review-results.md. Technical results do not establish final aesthetic approval.

The generator takes the original semantic profile, not reference mesh, UV, adjacency, pixels or character names. See review-protocol.md for the natural-language-to-recipe workflow. Rejected experiments are not silently declared successful: candidate 173’s ineffective local normal-weighting change was removed.
