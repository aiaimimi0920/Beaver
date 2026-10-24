# Source ear-fan quality policy (development)

This is a shared generation policy, not a character-name or exported-vertex
patch. It is still under native cross-composition and visual review.

## Construction and acceptance

The legacy first-pass option distributes five lower-root sectors by angular arc
length on the existing analytic membrane curve. Root and outer-rim endpoints
remain anchored. The authored report records `angular-source-fan-v1`; legacy
or untagged geometry cannot enter the new unchanged-fan acceptance path.

The complete, connected, single-region/material fan is measured from current
coordinates. Acceptance requires finite coordinates/normals, positive area,
normalized normals, quality at least .35, minimum angle at least 12 degrees,
maximum angle at most 145 degrees, pair-normal dot at least sqrt(.5), and
vertices within the existing 12 mm root neighborhood. These are explicit
engineering budgets, not universal industry standards.

If the tagged fan fails after attachment fairing, bounded in-generation
resampling may adjust its interior stations. Displacement extends harmonically
through at most two eligible interior rings, keeping skin/rim boundaries fixed.
No vertices or faces are added or deleted. Maximum displacement is 1.5 mm.
UV interpolation uses double-precision source coordinates before mesh storage;
all affected loops must have compatible region/material and continuous UVs.

Every affected neighboring triangle must retain quality at least
min(previous quality, .35), minimum angle at least min(previous minimum, 12°),
and maximum angle at most max(previous maximum, 145°), within documented
floating-point tolerances. Positive area and orientation are checked separately.
Already-poor neighbors cannot be made worse. Backtracking tries bounded source
fractions; failure remains a failure rather than weakening a threshold.

Changes stay in a private BMesh until both distinct sides are accounted for.
The mesh is freed on failure. Reports distinguish measured unchanged fans,
actual diagonal flips and bounded reflow displacement. In particular, the
`all_vertex_positions_unchanged` field of the diagonal stage does not describe
preceding generation/fairing stages. Final welded-seam, morph, SDF and visual
checks remain separate and mandatory for their respective acceptance claims.

## Game-budget source construction

The shared game-budget default uses `quality-budget-source-fan-v2`. A deterministic
129-sample maximin dynamic program chooses three lower-fan sectors and the same
source stations on the next membrane row. The score covers the fan and both
legal first-strip diagonals, rejecting triangles outside the angle budgets.
A monotone quality-selected zipper joins the four-point row to the ordinary
six-point next row. Root and rim endpoints stay fixed. This is source topology
construction, not an exported mesh collapse or a fitted character-coordinate list.

The independent final gate is unchanged. Current strict Faye trial52 produces
4,164 whole-Face triangles and passes both ears without reflow. Public-default
blunt-chin/lip-band/elongated-nose trial55 produces 4,188 and also passes. Neither
result alone establishes visual, collision or all-combination acceptance. See
GAME-FACE-BUDGET.md for current limitations.

## Historical evidence and limits

- Candidate31: the previously failing blunt-chin/lip-band/elongated-nose
  composition generated, packaged, built SDF and captured 36 actual views.
  Both five-triangle source fans were accepted without flips; minimum qualities
  were .381/.357 and minimum angles 13.24°/12.29°. Review found coherent attached
  ears, but a lower-root shading notch and other face shading concerns remain.
- Candidate43: the controlled Faye specimen generated and built SDF at 8,056
  triangles. The right-side reflow selected 12 interior vertices with maximum
  displacement .565 mm; root/rim endpoints and topology remained fixed. Final
  fan qualities were .398/.416, with neighboring-quality budgets satisfied.
  Its new matched renders and further composition regression remain pending.

Earlier proposals that failed source, orientation, neighboring-quality or
numeric checks were not treated as successful models. These discrete examples
are not proof for every face/ear combination, continuous animation, collision
freedom, zero warnings or final aesthetic acceptance.
