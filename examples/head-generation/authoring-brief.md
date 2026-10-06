# Face refinement acceptance specification after candidate 102

User review, 2026-10-06, 16:38 UTC+8. Baseline 102 remains recoverable.

## Authored generation requirements

- Generate an original stylized NPR head from semantic proportions and parameters. Do not load reference meshes, textures or UVs in the generation recipe. A character name is not an executable modeling specification.
- Cheek planes must transition continuously into the lower face. Remove unintended horizontal ridges and alternating concave/convex bands, while preserving local feature support.
- The lateral boundary from ear root toward the top of the head must describe a continuous, gentle arc without accidental dents or bulges.
- The whole jaw boundary must taper smoothly toward the chin, forming a curved triangular silhouette rather than a broad, flat-bottomed trapezoid. Do not achieve this by pulling a single chin vertex or collapsing edge loops to a pole.
- Preserve independently controlled shading transitions at the facial centerline and a subtle lower-jaw boundary. Maintain smooth cheek shading between these semantic boundaries. Distinguish subdivision edge crease, geometric dihedral and corner-normal discontinuity. Do not harden every edge.
- Eyes require a deliberate rim/transition around their circular rear structure, shallow embedded placement, independent iris, and visible original white catchlight. Inspect front, both obliques, both sides and rear. A depth reduction alone is insufficient.
- Ear shape, root continuity and shallow open membrane must be judged against the reference appearance and structure. Use original procedural detail; do not copy reference texture data.
- Refine oral cavity cross-section, posterior contour and inner-lip connection as a coherent structure; retain a narrow neutral lip seam, actual cavity and tongue, and localized mouth movement with stable chin.

## Verification

1. Inspect reference geometry, normals and rendered behavior separately. The unpacked FBX cannot establish original Blender modifier history; exported splits are not automatically errors.
2. Revise prompt/spec before changing authored parameters or code. Generate through the normal Beaver workflow.
3. Compare the candidate and reference at identical scale, view and lighting; include underside jaw and inverted/oblique cheek views from the user's review.
4. Keep geometry winding, degeneracy, semantic crease and mouth-motion checks. Add regional normal and contour checks where they measure the requested property.
5. Technical test success is distinct from visual acceptance. Preserve each useful verified source checkpoint remotely and keep runnable model artifacts recoverable.

## Diagnostic evidence, not yet a proven complete cause

Candidate 102 calculates area-weighted per-vertex normals after final triangulation and assigns them to smooth face corners. This averages across a shared facial centerline. The reference export contains differing corner normals on several anterior midline edges, in addition to geometric dihedral. This supports testing regional normal partitioning, without claiming that a Blender Crease attribute alone explains the observed rendering.

Candidate 102 also blends sparse horizontal cross sections and applies a lateral jaw-return correction. Their curvature and side-boundary derivatives require independent inspection before changing the cheek; global smoothing or normal-only masking is not sufficient.

## Additional rendering requirement

Outline is a shader-rendered effect. Its visibility and continuity at normal front/oblique angles must be compared at matched scale and lighting. Diagnose actual material settings, outline width, normal inputs and depth/culling behavior. Do not add black geometry as a substitute. Candidate 102 has some visible outline in the inspected front render, so the defect is inconsistent visibility/continuity, not a verified total absence. User resumed inspection after initially handing the screen back; defer UI input until the viewing interval is over.

## Candidate 103 isolation
First test left/right skin corner-normal partition at the shared facial centerline, while retaining area-weighted smooth normals within each side. Other geometry stays at baseline so the change can be inspected without conflating curvature and shading.
