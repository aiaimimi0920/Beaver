from paths import candidate_root, validation_root
from pathlib import Path
import numpy as np, json, itertools, collections, sys

V = Path(str(validation_root()))
R = V / str(candidate_root())
A = np.load(R / "full-fields.npz")
N = np.load(R / "region.npz")["faces"]
uv = A["TEXCOORD_1"]
old = np.load(V / "asy-proposal-pairs-joint-rebuild-20261112/full-geometry.npz")[
    "faces"
]
scope = [
    x["face"]
    for x in json.loads(
        (V / "atd-band-protected-constraint-audit-20261112/SCOPE.json").read_text()
    )
]
protected = json.loads(
    (V / "atd-band-protected-constraint-audit-20261112/CONSTRAINTS.json").read_text()
)["shadow_protected_faces"]
base = A["POSITION"]
D = np.array([A[f"morph{k}.POSITION"] for k in range(7)])
flips = 0
minarea = 1000000000.0
maxasp = 0.0
for w in itertools.product([0, 0.5, 1], repeat=7):
    p = base + np.einsum("m,mvc->vc", w, D)
    q = p[N]
    n = np.cross(q[:, 1] - q[:, 0], q[:, 2] - q[:, 0])
    q32 = q.astype(np.float32)
    n32 = np.cross(q32[:, 1] - q32[:, 0], q32[:, 2] - q32[:, 0])
    area = np.linalg.norm(n, axis=1)
    flips += int(np.sum(np.sum(n * n32, axis=1) <= 0))
    minarea = min(minarea, float(area.min()) * 1000000.0)
    edge = np.linalg.norm(q - np.roll(q, 1, axis=1), axis=2)
    maxasp = max(maxasp, float(np.max(edge.max(1) ** 2 / np.maximum(area, 1e-30))))
E = collections.Counter(
    (
        tuple(sorted((int(a), int(b))))
        for t in old[scope]
        for a, b in zip(t, np.roll(t, -1))
    )
)
used = np.unique(N)
tj = []
for (a, b), n in E.items():
    if n != 1:
        continue
    d = uv[b] - uv[a]
    tt = (uv[used] - uv[a]) @ d / (d @ d)
    dist = np.linalg.norm(uv[used] - uv[a] - tt[:, None] * d, axis=1)
    h = used[(tt > 1e-08) & (tt < 1 - 1e-08) & (dist < 1e-10)]
    if len(h):
        tj.append(dict(old_interface=[a, b], new_interior_nodes=h.tolist()))
from shapely.geometry import Polygon

pp = {f: Polygon(uv[old[f]]) for f in protected}
cross = []
for i, t in enumerate(N):
    p = Polygon(uv[t])
    fs = [f for f, q in pp.items() if p.intersection(q).area > 1e-15]
    if len(fs) > 1:
        cross.append(dict(candidate_triangle=i, protected_faces=fs))
rep = dict(
    poses=2187,
    float32_nonpositive_instances=flips,
    min_doublearea_mm2=minarea,
    max_aspect=maxasp,
    unsplit_interface_edges=tj,
    triangles_crossing_protected_original_edges=cross,
)
(R / "VALIDATION.json").write_text(json.dumps(rep, indent=2))
print(rep)
