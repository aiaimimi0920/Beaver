# Anime NPR character workflow

Use this reference for anime-npr-character / 二次元 NPR 人物制作规范.
The target is an editable game character with illustration-like proportions,
clean color regions and deliberate face shading. Derive proportions from the
user's actual reference. For a Silver Wolf-led task, use that reference's facial
relationships; do not import the enlarged heads, eyes or narrowed necks of a
different tutorial character. Explicit user requests for other proportions win.

## Reference and camera

Record a small set of normalized relationships: head/body scale, eye opening and
spacing, cheek and jaw widths, chin length, nose/lip projection, neck width and
attachment. Separate observed facts from artistic choices. Keep character-specific
values in task notes rather than making them universal skill defaults.

Use orthographic views for alignment and measurement, and the intended game
camera's perspective and distance for appearance. Check front, three-quarter and
profile; changes in one view must survive the others. Inspect the face without
hair before using the hairstyle as context. Resolve cheek-to-chin taper and both
chin-to-throat and occiput-to-nape contours before detailing the face.
For chin revisions, measure several low jaw cross-sections, the central chin pad,
its forward projection and its underside. Carry a rounded transition through the
center seam and adjacent jaw strips; concentrating them into one pointed pole can
leave a pinched chin even when the overall head width matches the reference.

## Head shell and neck boundary

Measure forehead, cheek and jaw depth as well as frontal width. Compare the same
anatomical surfaces; the bounds of an open facial shell cannot describe a complete
skull. Preserve rounded facial planes in oblique views and check geometric shading
before applying a broad frontal normal field that can flatten their appearance.
Reject sampled rays that hit an eye cavity, a grazing edge or a rear surface when
reconstructing the cheek. Recheck the resulting planes after interpolation.

When the chosen reference uses an open posterior face/ear shell, retain that
construction. Trace its real border from the underside of the chin through the
jaw and ear roots to the upper opening. Keep the curved jaw boundary and the neck
as separately editable surfaces, with an appropriate concealed attachment overlap.
Check both sides, profile and an exposed rear view. A painted line cannot validate
the required opening or attachment. Do not close intended boundaries merely to
pass a generic watertight-mesh check.

Check coverage with the actual hairstyle and required motion. Uncovered scalp or
rear views need corresponding visible surfaces. Keep construction choices scoped
to the reference and character; record untested deformation limits explicitly.

## Direct topology and half-model construction

Default to direct mid/low-poly authoring. Spend geometry on silhouette, overlap,
necessary thickness and deformation. Dense subdivision or baked micro-normal
detail is not a default solution for an unresolved anime face.

For the face, head/neck, body, limbs and symmetric clothing foundations:

1. Establish the center plane and keep center vertices on it. Enable clipping
   and merging where appropriate; do not merge intended openings or adjacent layers.
2. Author one side with a live Mirror modifier so the complete form is visible
   throughout editing. Inspect the center seam, surface normals and full silhouette.
3. Establish quad-dominant eye/mouth loops and coherent cheek, jaw, neck and joint
   flow. Place poles away from important deformation paths. Use the topology
   checks below without prescribing universal loop counts.
4. Approve the symmetric foundation before asymmetry. Preserve the editable half
   source; generate the complete mesh when required by asymmetric edits, shape
   keys, rigging or export. Decide modifier order against the actual toolchain.
5. Add scars, directional patterns, unequal ornaments and other intended differences
   with independent geometry or UV/mask space. Verify mirrored weights, normals,
   UV orientation and expressions rather than assuming geometry mirroring covers them.

Hair is excluded from the default whole-part symmetry workflow. Design its
silhouette and arrangement independently. Reuse individual locks only when useful
and adjust them to the intended hairstyle. Eye geometry may share a foundation;
gaze, highlights and directional iris details need deliberate bilateral control.

Mirroring reduces editing and maintenance work. The exported complete mesh still
has its full rendering cost; report evaluated/exported counts, not only half-cage
counts. Keep editing groups separate from the package's final mesh grouping.

## Editable quads and export triangulation

Keep the editable face, ears, mouth support and other deforming foundations
predominantly quadrilateral. Build continuous loops around openings and use quad
patches for transitions and closures. Avoid automatic center fans on the chin,
ear bowl, forehead or lip corners. Circular, non-deforming eye layers can retain
a deliberate center fan, with quad annuli around it where useful.

Check edge flow and surface quality as well as polygon counts. Remove unintended
N-gons, collapsed edges, folded or severely twisted quads and abrupt density jumps.
Distribute corner transitions and keep high-valence poles away from eyelid/lip
creases and exposed silhouettes. Pairing arbitrary triangles into quads can leave
the same bad flow. Imported triangulated references can suggest the original
loops, but the original quad pairing cannot always be recovered uniquely.

Game export and GPU rendering use triangles. Preserve the quad source, inspect
the chosen triangulation on an evaluated/export copy, and reimport the actual
asset to check normals, silhouette, UVs and layer intersections. Quads alone do
not guarantee good triangulation or deformation. Include a true-edge wire view
alongside shaded views; report authored quads/triangles separately from exported
triangle counts. Validate the required expressions when they are in scope.

## Surface meshes and budgets

Author primarily visible surfaces and thin shells. Curved surface meshes still
need the volume that creates the face/body silhouette. Avoid duplicate internal
layers, unnecessary caps and blanket Solidify modifiers. Keep local thickness at
visible rims, eyelids, mouth openings, collars, cuffs and relevant side profiles.

Brows, lashes and suitable decorations can use tapered ribbons or texture detail.
Hair can use shaped strips with readable cross-sections. Remove occluded body
geometry only after checking the required motion, cameras and clothing variants.
Preserve useful mouth/eye interiors and coverage; indiscriminate deletion can
expose holes during animation.

Choose geometry and texture density for target viewing distance and platform.
Avoid fixed universal triangle limits, UV percentages, texture sizes or millimeter
offsets. Track material splits, transparency overlap, two-sided shading, shadows
and draw calls alongside triangles. Many overlapping transparent cards can be
expensive. Verify real engine behavior before claiming a performance improvement.
After simplification, recheck narrow trims, UV boundaries and intersecting layers.
Preserve or retopologize a sensitive region when automatic reduction damages it.

## Layered facial features

Build a real eyelid aperture, inward rim and recessed scleral support surface.
Fit iris, pupil and restrained catchlight layers to that eye support, behind the
covering upper lid. Choose geometry or texture for a layer according to the
reference and animation needs; avoid excessive concentric subdivisions. Remove
duplicate painted pupils/glints when introducing equivalent geometry.
Measure the support depth, iris curvature, lid contour and lid overlap separately.
An iris can retain its own shaped surface within a deeper white socket. Correct
visible layer intersections locally while preserving that form; account for the
parts intentionally covered by lids when interpreting clearance measurements.
When eyes appear too far outward, distinguish lateral spacing from forward
protrusion. Compare inner/outer corners, iris centers, depth relative to the
surrounding face and upper-lid coverage at the same head scale. Reposition the
socket, lid and eye layers coherently; an isolated iris shift can break the gaze
or create new intersections. Do not add large lateral shifts when measurements
instead identify a depth or aperture-shape problem.

Judge recession by overlap, rim depth and oblique parallax. Check eye corners,
the far eye, coplanar flicker and skin coverage. Do not promise automatic eye
tracking merely from using concave surfaces or force realistic spheres into a
reference that uses a different construction.
Fit overlays against the evaluated low-poly surface: a nominal offset from an
analytic surface can still leave a pupil or iris behind the rendered triangles.
Check triangle winding with backface culling enabled. Recalculating normals on
disconnected open ribbons or disks can leave them facing inward; custom shading
normals do not repair winding. Verify each visible layer and its exported copy.

Keep three distinct eye accents: upper eyebrow, subtle upper-lid crease and
eyelashes attached to the lid edge. The middle accent can be carried by the skin
texture or a conforming shallow ribbon. Keep it lighter and narrower than brows
and lashes; taper the accents and keep the lower lid quiet. Check lash roots and
rising tips against the skin in oblique views.
Fit brows, crease ribbons and lash roots to the evaluated skin triangles as well;
an analytic skin approximation can leave floating fragments or buried accents.

Give the mouth a lip opening and inward lining with sufficient interior coverage.
Where expressions need them, add curved dental ribbons and a recessed tongue.
Let the resting lips cover these parts naturally; do not widen the smile solely
to display the interior. Inspect the normal lip profile and an isolated cutaway.
Use facial transitions and a fine slit instead of a uniform raised tube.
Measure upper/lower lip projection, the center and corner heights, and the opening
in the resting expression. Rebuild local perioral support with the lip contours;
an inward mouth cavity alone does not establish the visible lip shape.

Place ears using height, fore-aft position, tilt and root attachment together.
Trace the helix, concha and lobule from the reference and inspect the root in
profile. Use a shaped shell and coherent attachment when the reference has them;
matching only an oval's frontal size leaves its depth and placement unresolved.
Coordinate subtle inner-ear color with the geometry so both structural and NPR
views retain the intended cues.
Give the ear a usable UV island or an equally controlled texture region, with
padding and sufficient texel density at the intended view size. Paint the helix,
antihelix branches, concha and lobule deliberately with restrained transitions
that follow their geometry. A few constant-color swatches do not describe these
structures. Check the mapped ear close-up, at game distance and with the hairstyle;
increasing texture resolution alone does not repair incorrect mapping or anatomy.

Test required blinks, smiles and mouth opening through intermediate states, not
only endpoints. Check eyelids, lashes, iris, teeth and tongue together. An editable
base without tested expressions must be labelled as such, not animation-ready.

## Normals, color and shadow design

Develop a minimal valid NPR material while building the foundation. Coordinate
surface normals, face-shadow maps/masks, clean base colors and controlled shadow
hues with the target shader. Do not make every eye part unlit or emissive by rule.
Use gradients, blush and iris/hair detail at a density that survives filtering.

An editable normal proxy is an option: duplicate the face, shape its shading
surface and transfer custom face-corner normals through restricted weights.
Topology mapping requires actual topology and corner correspondence; equal vertex
counts or similar density are insufficient. Choose proxy shape and transfer
strength from the result. Protect ears, lashes, lid boundaries, mouth interior
and useful nose transitions. A proxy does not replace all face-shadow controls.

Inspect exported normals and shader response with changing light directions and
the required expressions. Separate a shape defect from a normal, UV or material
defect before editing geometry. Do not bake Unity render queue numbers into a
Godot workflow. Any stylized hair-shadow surface or eyebrow visibility exception
must respect intended depth, camera angles and hair motion; never reveal facial
features through the back of the head or unrelated occluders.

Vary outline width/color by region and check at game distance. Avoid duplicate
outlines when the engine already supplies them, excessive internal facial lines
and heavy hair tips. Test UV padding, orientation and mipmap behavior.

## Review and reusable lessons

Work from reference/proportion to symmetric foundation, layered features,
asymmetry, then the required expression and game-rendering checks. Use a small
matched set of structural and NPR views. Keep source and evaluated/exported
topology counts distinct and identify intentionally open shell boundaries.

For human-guided rounds, implement only the current revision, save an independent
candidate and a labelled side-by-side reference scene, open the saved comparison
and stop. Preserve earlier versions and the user's open scene. Keep references
out of production exports. Report technical checks separately from human visual
acceptance; a low triangle count or successful import does not prove beauty.

Generalize methods and observed checks into the skill. Keep unreviewed artistic
results marked pending and record actual feedback in task notes before presenting
a particular recipe as validated. Do not encode unavailable diffusion/NeRF training
controls as if they were commands in the Codex + Blender workflow.

## Method references

The following are method references, not universal commercial specifications:

- [User-supplied synthesis](https://docs.google.com/document/d/1PKOml3aWjbRascBmK2pziRjhy4XfE6qBlGTHrDUNrDE/edit)
- [Normal proxy and hair-shadow tutorial notes](https://vtuber-hudan.hatenablog.com/entry/Blender_FusakoIsMyGod_Unity2)
- [Unity material tutorial notes](https://vtuber-hudan.hatenablog.com/entry/Blender_FusakoIsMyGod_Unity3)
