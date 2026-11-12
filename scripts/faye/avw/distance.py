from paths import candidate_root, validation_root
from pathlib import Path
import numpy as np, json

V = Path(str(validation_root()))
R = V / str(candidate_root())
A = dict(np.load(V / "asy-proposal-pairs-joint-rebuild-20261112/full-fields.npz"))
B = dict(np.load(R / "full-fields.npz"))
T = np.load(V / "asy-proposal-pairs-joint-rebuild-20261112/full-geometry.npz")["faces"]
ids = np.array(
    [
        z["face"]
        for z in json.loads(
            (V / "atd-band-protected-constraint-audit-20261112/SCOPE.json").read_text()
        )
    ]
)
FA = T[ids]
FB = np.load(R / "region.npz")["faces"]
from closest_surface import closest

bary = np.array(
    [
        [1, 0, 0],
        [0, 1, 0],
        [0, 0, 1],
        [1 / 3] * 3,
        [0.5, 0.5, 0],
        [0, 0.5, 0.5],
        [0.5, 0, 0.5],
    ]
)


def pts(p, F):
    return np.einsum("ki,fic->fkc", bary, p[F]).reshape(-1, 3)


rep = []
for label, w in [
    ("neutral", [0] * 7),
    ("open", [1, 0, 0, 0, 0, 0, 0]),
    ("open_spread", [1, 0, 0, 0, 1, 0, 0]),
    ("round_pucker", [1, 0, 0, 1, 0, 1, 0]),
]:
    pa = A["POSITION"] + sum((w[k] * A[f"morph{k}.POSITION"] for k in range(7)))
    pb = B["POSITION"] + sum((w[k] * B[f"morph{k}.POSITION"] for k in range(7)))
    rr = []
    for name, src, Fsrc, dst, Fdst in [
        ("fixed_source_to_candidate", pa, FA, pb, FB),
        ("candidate_to_source", pb, FB, pa, FA),
    ]:
        sample = pts(src, Fsrc)
        near = closest(sample, dst, Fdst)
        d = np.array([q[0] for q in near]) * 1000
        rr.append(
            dict(
                direction=name,
                samples=len(sample),
                max_mm=float(d.max()),
                rms_mm=float(np.sqrt(np.mean(d * d))),
                worst_source_face=(
                    int(ids[d.argmax() // 7]) if name.startswith("fixed") else None
                ),
            )
        )
    rep.append(dict(pose=label, distances=rr))
samples = np.einsum("ki,fic->fkc", bary, A["TEXCOORD_1"][FA]).reshape(-1, 2)
target = B["TEXCOORD_1"][FB]
m = np.stack([target[:, 1] - target[:, 0], target[:, 2] - target[:, 0]], -1)
inv = np.linalg.inv(m)
errs = {k: 0.0 for k in A}
miss = 0
for i, x in enumerate(samples):
    z = np.einsum("fij,fj->fi", inv, x - target[:, 0])
    bc = np.c_[1 - z.sum(1), z]
    ok = np.where(bc.min(1) > -1e-08)[0]
    if not len(ok):
        miss += 1
        continue
    j = int(ok[0])
    f = i // 7
    w = bary[i % 7]
    for k in A:
        errs[k] = max(
            errs[k], float(np.max(np.abs(w @ A[k][FA[f]] - bc[j] @ B[k][FB[j]])))
        )
(R / "SAMPLED-SURFACE-AND-FIELDS.json").write_text(
    json.dumps(
        dict(
            note="finite samples, not continuous bound; source ASY not production default",
            surface=rep,
            fixed_uv_field_component_max=errs,
            unmatched=miss,
        ),
        indent=2,
    )
)
print(
    json.dumps(
        dict(
            surface=rep,
            position_component_mm=errs["POSITION"] * 1000,
            mouthopen_component_mm=errs["morph0.POSITION"] * 1000,
            uv0=errs["TEXCOORD_0"],
            unmatched=miss,
        )
    )
)
