# Reproducible head recipe, iteration 145

Natural-language specification → semantic parameters and original procedural topology → Beaver generation → native MiDot/NPR comparison. The generator does not load the reference mesh, UVs or texture pixels. The reference is read-only diagnostic material, never included in this source submission.

This batch implements the user's candidate136 feedback: a smoother posterior jaw boundary; multiview open wrapped ears and softer original ear pigment; fuller rounded eye contours with separate eyebrow, eyelid crease, upper lash and outer liner; larger simple eye-pocket/oral rear sections; and small adjustable chin follow. The accepted center-face normal division is retained.

The ear-root black wedge required three distinct corrections: the lower front sheet must not sweep anterior to its actual attachment; its free return must face the reverse lateral side; and the return must stay behind the local curved basin at oblique views. Candidate145's inspected ±45-degree and right90-degree NPR views no longer show the earlier wedge. This is sampled visual verification, not proof over every camera angle.

58 pure Python tests pass, all13 generated source/spec hashes match, and native MiDot asset validation passes. The native comparison is actually running. MouthOpen0,0.5,1 were exercised; front chin-tip full-open drop is1.500010mm, filtered lower chin skin maximum1.539469mm. Body/Hair primitives and the pinned runtime/framework remain unchanged.

Candidate145 project aggregate passed7tests with0failures at2026-10-06T16:40:08UTC. The user's manual aesthetic confirmation remains pending. Full-character wardrobe/animation, release and production deployment are outside this head-review checkpoint. Hosted CI remains a separate merge gate; keep the PR draft. Run unittest discovery in this directory. bundle.py packages a request for Beaver blender.start but never executes Blender itself.
