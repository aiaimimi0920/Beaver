from paths import candidate_root, validation_root
from pathlib import Path
import numpy as np, json

V = Path(str(validation_root()))
R = V / str(candidate_root())
A = dict(np.load(V / "asy-proposal-pairs-joint-rebuild-20261112/full-fields.npz"))
B = dict(np.load(R / "full-fields.npz"))
F = np.load(R / "region.npz")["faces"]
uv = B["TEXCOORD_1"]
T = np.load(V / "asy-proposal-pairs-joint-rebuild-20261112/full-geometry.npz")["faces"]
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
m = np.stack([uv[F[:, 1]] - uv[F[:, 0]], uv[F[:, 2]] - uv[F[:, 0]]], -1)
inv = np.linalg.inv(m)
err = {k: 0.0 for k in A}
per = []
faces = [5445, 5448, 5449, 5451, 5454, 5428, 5452, 5453, 5456, 5455, 5436]
for f in faces:
    local = {k: 0.0 for k in A}
    for w in bary:
        q = w @ uv[T[f]]
        bc = np.einsum("fij,fj->fi", inv, q - uv[F[:, 0]])
        ww = np.c_[1 - bc.sum(1), bc]
        ix = np.where(ww.min(1) > -1e-08)[0]
        i = ix[0]
        for k in A:
            e = float(np.max(np.abs(ww[i] @ B[k][F[i]] - w @ A[k][T[f]])))
            err[k] = max(err[k], e)
            local[k] = max(local[k], e)
    per.append(dict(face=f, errors=local))
rep = dict(
    endpoint7055_exact_fields={
        k: bool(np.array_equal(A[k][7055], B[k][7055])) for k in A
    },
    faces=faces,
    samples=77,
    component_error=err,
)
(R / "MOTION-DOMAIN-FIELD-CHECK.json").write_text(json.dumps(rep, indent=2))
print(rep)
(R / "PER-DOMAIN-FIELDS.json").write_text(json.dumps(per, indent=2))
print(
    [(x["face"], x["errors"]["POSITION"], x["errors"]["morph0.POSITION"]) for x in per]
)
