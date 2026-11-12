from paths import candidate_root, validation_root
from pathlib import Path
import json, numpy as np

R = Path(str(candidate_root()))
A = dict(np.load(R / "full-fields.npz"))
U = A["TEXCOORD_1"]
hits = json.loads((R / "INTERNAL-T-JUNCTIONS.json").read_text())
report = []
for h in hits:
    a, b = h["edge"]
    for v in h["nodes"]:
        t = np.dot(U[v] - U[a], U[b] - U[a]) / np.dot(U[b] - U[a], U[b] - U[a])
        errors = {
            k: float(np.max(abs(arr[v] - (1 - t) * arr[a] - t * arr[b])))
            for k, arr in A.items()
        }
        report.append(dict(edge=[a, b], node=v, t=float(t), errors=errors))
(R / "INTERFACE-FIELD-CHECK.json").write_text(json.dumps(report))
print([(r["edge"], r["node"], max(r["errors"].values())) for r in report])
assert max((max(r["errors"].values()) for r in report)) < 1e-10
for file in ["region.npz", "full-geometry.npz"]:
    d = dict(np.load(R / file))
    faces = d["faces"].tolist()
    for h in hits:
        a, b = h["edge"]
        assert len(h["nodes"]) == 1
        v = h["nodes"][0]
        out = []
        for tri in faces:
            if a in tri and b in tri:
                for i in range(3):
                    x, y, z = (tri[i], tri[(i + 1) % 3], tri[(i + 2) % 3])
                    if {x, y} == {a, b}:
                        out.extend([[x, v, z], [v, y, z]])
                        break
            else:
                out.append(tri)
        faces = out
    d["faces"] = np.array(faces)
    np.savez(R / file, **d)
print("split all affine interfaces")
