# Eye-accent candidate185 verification

2026-10-07. This is a new review candidate, not final artistic approval.

The owner requested layered upward-swept upper lashes, a curved branched outer accent, an independent lower-outer tuft, and roots close to the eyelid when viewed from above. The recipe now builds these as original parameterized thin sheets and fits their returns against the generated eyelid/evaluated skin. It does not load reference geometry or textures.

The former global upper-ink plane had a sampled7.8–9.4mm stand-off. Root placement now follows the existing generated lid. Subsequent oblique checks corrected excessive corner depth extrapolation. A thin-wall preflight caught a mismatched back chord crossing the curved front; six-rail matched front/back sampling fixes this and has a regression test.

Verified:98 pure tests;14/14 source manifest hashes; actual Beaver generation exit0; native MiDot preview pass without engine errors; current-head integration9/9 tests,178 assertions, GUT9.4.0, exit0. Both head-review test files target185 and match the validated snapshot. Native baseline UID-fallback warnings remain; this is not a zero-warning claim.

The actual running comparison was inspected at front, both45-degree views, right90-degree, and32.5-degree top view, using white/wire/render modes. The former large upper-band gap and extreme outer spear are reduced. Thin side branches and the isolated lower tuft are present. The reference and candidate do not have identical white-mode shading or silhouettes; final visual satisfaction remains the owner's decision. Existing accepted head, eyes/pupils, nose, mouth and ears retain their authoring sources/parameters.

Protected-region strict export comparison is not a byte-identity pass. Earlier diagnostics exposed tiny regenerated normal/tangent/morph differences despite unchanged authoring sources and positions. Do not state that all exported attributes are identical. The raw failed strict checks are retained in the private validation evidence; they were not used to change protected geometry or silently relabeled as passing.

Generated185 GLB SHA256:cc1831d51f5d8e7425054a92b98f90785d827506a293c150a730df6e547e21e3.

Source publication and durable private artifact backup are separate steps. No hosted-CI, merge, release, full-wardrobe or production-deployment acceptance is implied.
