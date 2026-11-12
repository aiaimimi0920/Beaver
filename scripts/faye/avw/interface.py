from paths import candidate_root, validation_root
from pathlib import Path
import numpy as np, json, sys

V = Path(str(validation_root()))
R = V / str(candidate_root())
C = dict(np.load(R / "full-fields.npz"))
uv = C["TEXCOORD_1"]
old = np.load(V / "asy-proposal-pairs-joint-rebuild-20261112/full-geometry.npz")[
    "faces"
]
scope = {
    x["face"]
    for x in json.loads(
        (V / "atd-band-protected-constraint-audit-20261112/SCOPE.json").read_text()
    )
}
N = np.load(R / "region.npz")["faces"]
check = json.loads((R / "VALIDATION.json").read_text())
nodes = np.array(
    sorted(
        {v for x in check["unsplit_interface_edges"] for v in x["new_interior_nodes"]}
    )
)
import triangle

outside = []
split = []
fields = {k: list(v) for k, v in C.items()}
audit = []
for fi, t in enumerate(old):
    if fi in scope:
        continue
    ids = []
    for a, b in zip(t, np.roll(t, -1)):
        d = uv[b] - uv[a]
        dd = d @ d
        if dd < 1e-24:
            ids.append(int(a))
            continue
        tt = (uv[nodes] - uv[a]) @ d / dd
        dist = np.linalg.norm(uv[nodes] - uv[a] - tt[:, None] * d, axis=1)
        hits = []
        for j in np.flatnonzero((tt > 1e-08) & (tt < 1 - 1e-08) & (dist < 1e-10)):
            v = int(nodes[j])
            p = float(tt[j])
            vals = {k: (1 - p) * C[k][a] + p * C[k][b] for k in C}
            err = max(
                (
                    float(np.max(abs(vals[k] - C[k][v])))
                    for k in ["POSITION"] + [f"morph{i}.POSITION" for i in range(7)]
                )
            )
            if err > 1e-08:
                continue
            attr = max((float(np.max(abs(vals[k] - C[k][v]))) for k in C))
            if attr > 1e-08:
                v = len(fields["POSITION"])
                for k in fields:
                    fields[k].append(vals[k])
            hits.append((p, v))
        ids.append(int(a))
        ids.extend((v for p, v in sorted(hits)))
    if len(ids) == 3:
        outside.append(t)
        continue
    coords = np.array(fields["TEXCOORD_1"])[ids]
    n = len(ids)
    res = triangle.triangulate(
        dict(vertices=coords, segments=np.array([[j, (j + 1) % n] for j in range(n)])),
        "pYQ",
    )
    assert len(res["vertices"]) == n
    new = np.array(ids)[res["triangles"]]
    split.extend(new)
    outside.extend(new)
    audit.append(dict(original_face=fi, new_triangles=len(new)))
A = {k: np.array(v) for k, v in fields.items()}
allT = np.vstack([outside, N])
p = A["POSITION"]
nn = np.cross(p[allT[:, 1]] - p[allT[:, 0]], p[allT[:, 2]] - p[allT[:, 0]])
flip = np.sum(nn * A["NORMAL"][allT].mean(1), axis=1) < 0
allT[flip] = allT[flip][:, [0, 2, 1]]
np.savez(R / "full-fields.npz", **A)
np.savez(R / "full-geometry.npz", faces=allT)
np.savez(R / "interface-splits.npz", faces=np.array(split))
(R / "INTERFACE-SPLITS.json").write_text(json.dumps(audit, indent=2))
print(
    dict(
        outside_faces_split=len(audit),
        new_outside_triangles=len(split),
        attribute_side_rows_added=len(A["POSITION"]) - len(C["POSITION"]),
    )
)
