from paths import candidate_root, validation_root
from pathlib import Path
import numpy as np, json, itertools, collections

V = Path(str(validation_root()))
R = V / str(candidate_root())
A = np.load(R / "full-fields.npz")
F = np.load(R / "full-geometry.npz")["faces"]
N = np.load(R / "region.npz")["faces"]
split = np.load(R / "interface-splits.npz")["faces"]
Q = np.vstack([N, split])
base = A["POSITION"]
D = np.array([A[f"morph{k}.POSITION"] for k in range(7)])
bad = 0
areaMin = 1.0
aspect = 0.0
for w in itertools.product([0, 0.5, 1], repeat=7):
    p = base + np.einsum("m,mvc->vc", w, D)
    t = p[Q]
    n = np.cross(t[:, 1] - t[:, 0], t[:, 2] - t[:, 0])
    s = t.astype(np.float32)
    n32 = np.cross(s[:, 1] - s[:, 0], s[:, 2] - s[:, 0])
    ar = np.linalg.norm(n, axis=1)
    bad += int(np.sum(np.sum(n * n32, axis=1) <= 0))
    areaMin = min(areaMin, float(ar.min()) * 1000000.0)
    ed = np.linalg.norm(t - np.roll(t, 1, axis=1), axis=2)
    aspect = max(aspect, float(np.max(ed.max(1) ** 2 / np.maximum(ar, 1e-30))))
nodes = np.array(
    sorted(
        {
            v
            for x in json.loads((R / "VALIDATION.json").read_text())[
                "unsplit_interface_edges"
            ]
            for v in x["new_interior_nodes"]
        }
    )
)
uv = A["TEXCOORD_1"]
E = {tuple(sorted((int(a), int(b)))) for t in F for a, b in zip(t, np.roll(t, -1))}
tj = []
for a, b in E:
    d = uv[b] - uv[a]
    dd = d @ d
    if dd < 1e-24:
        continue
    tt = (uv[nodes] - uv[a]) @ d / dd
    dist = np.linalg.norm(uv[nodes] - uv[a] - tt[:, None] * d, axis=1)
    for j in np.flatnonzero((tt > 1e-08) & (tt < 1 - 1e-08) & (dist < 1e-10)):
        v = int(nodes[j])
        p = tt[j]
        err = max(
            (
                float(np.max(abs(A[k][v] - ((1 - p) * A[k][a] + p * A[k][b]))))
                for k in ["POSITION"] + [f"morph{i}.POSITION" for i in range(7)]
            )
        )
        if err < 1e-08:
            tj.append([a, b, v])
rep = dict(
    changed_faces_checked=len(Q),
    poses=2187,
    float32_nonpositive_instances=bad,
    min_doublearea_mm2=areaMin,
    max_aspect=aspect,
    remaining_interface_geometric_tjunctions=tj,
)
(R / "FINAL-INTERFACE-DYNAMIC.json").write_text(json.dumps(rep, indent=2))
print(rep)
