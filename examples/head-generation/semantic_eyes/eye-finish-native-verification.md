# Illustrated eye finish verification, 2026-10-18

The bounded slice reduces excessive painted sclera occlusion and shortens the
illustrated upper-lash tail. Aperture calibration, spacing, iris dimensions,
face/ear/nose geometry and all native orientation/clearance guards stay unchanged.
Nonillustrated profiles retain their previous arithmetic.

## Fixed-condition diagnosis

Candidate 761 was captured at yaw 0 and +/-45 degrees with production shading
and diagnostic base color (mode 6), each with outline widths 1 and 0: 12 captures.
The gray sclera remained in base color. The recipe has authored upper occlusion
and corner tint, so that paint is an established contributor; this does not prove
that runtime lighting contributes nothing. The continuous lower line also has
an authored lash ribbon. The thin bright upper-lid strip remains unresolved.

## Native candidates

- 762: paint band depth .045 H -> .025 H; strength .32 -> .14; corner tint
  .13 -> .08. Generation `37a7b249-834f-438e-beaa-08f7f40c2d14`, SDF
  `f726f7ad-49e4-4a64-ac6e-096156d23fa7`, capture `blink-1792301433-2403`.
- 763: additionally illustrated lash tail ends at t=1.025 rather than 1.06,
  with matched Hermite span .065 and tail lift .012. The actual aperture corner
  is not raised. Generation `e29b00f6-d1b1-48c2-96da-10c5423bb39f`, SDF
  `04f673dc-0ada-48ef-bc6c-e2b7f372741c`, capture `blink-1792301970-07918`.
- Both completed 84 native render/white/edge captures with hashes, requested
  morph weights and unchanged source-model hashes checked. Candidate 763's
  analytic orientation minimum was 0.0591597395 across 10,980 triangles over
  blink and MouthOpen [0,1]. This is not an intersection proof.
- Inspected front, oblique and blink images show a modest cleaner finish,
  without obvious new damage. The upper bright strip is still visible.
  This is not final aesthetic acceptance or a universal face-quality guarantee.

## Code checks

115 semantic-eye and 35 facial-surface tests passed. Black, Python compilation
and diff whitespace checks passed. Four changed Python files have simple
nonblank/noncomment counts 102, 296, 150 and 35. Local official effective-line
scanner was blocked because Cargo is absent; remote CI must verify that check.
No third-party images or models are included in this change.

## Isolated secondary blade, candidate 764

The optional secondary_lash_scale control defaults to 1 (identity). Candidate
764 uses .78 with the otherwise unchanged reconstructed 763 controls. This
shrinks the independent child blade along the sampled parent midpoint, leaving
its root fixed. Main lash rails, aperture, iris and eyebrow calibration remain
unchanged. Existing 3D bow/flare offsets are not scaled.

Native generation job ab1625ee-ae4f-496f-ad05-1a4fc3cd0a15 and SDF job
692e5198-9352-400c-80e8-115e120cb58d succeeded. Runtime blend SHA-256:
1a1df328a7f8021308f32e49a81647c83d30b276c8be2c2108c52ed7e887f98c.
The analytic orientation minimum remains 0.059159739495675545 on 10,980
triangles. Capture set blink-1792305908-95443 contains 84 native views with
image hashes, source-model identity and requested morph weights verified.

Front, +/-45, profile, raised/lowered views, half/full blink, independent wink,
white and edge views were captured. Inspected images show a shorter secondary
tip, clearer primary/secondary hierarchy and no obvious new damage in the
reviewed views. This is a modest finish change, not final user acceptance or
proof of no intersections. The existing thin bright upper-lid strip remains.

117 semantic-eye tests and 35 facial-surface tests passed. Black and diff
whitespace checks passed. The new regression checks cover both illustrated
modes, root preservation, all unchanged non-tuft fields, relative scaling,
identity and invalid inputs. Local official effective-line scanner remains
blocked by missing Cargo; remote CI must verify it. No external reference
images or third-party model assets are committed.
