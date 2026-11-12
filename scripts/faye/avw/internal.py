from paths import candidate_root, validation_root
from pathlib import Path
import numpy as np, json

R = Path(str(candidate_root()))
A = np.load(R / "full-fields.npz")
F = np.load(R / "region.npz")["faces"]
U = A["TEXCOORD_1"]
vs = np.unique(F)
edges = {tuple(sorted((int(a), int(b)))) for t in F for a, b in zip(t, np.roll(t, -1))}
hits = []
for a, b in edges:
    d = U[b] - U[a]
    den = d @ d
    if den < 1e-25:
        continue
    tt = (U[vs] - U[a]) @ d / den
    dd = np.linalg.norm(U[vs] - U[a] - tt[:, None] * d, axis=1)
    ids = vs[(tt > 1e-07) & (tt < 1 - 1e-07) & (dd < 1e-11)]
    if len(ids):
        hits.append(dict(edge=[a, b], nodes=ids.tolist()))
(R / "INTERNAL-T-JUNCTIONS.json").write_text(json.dumps(hits, indent=2))
print(hits)
