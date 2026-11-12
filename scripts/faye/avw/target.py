from paths import candidate_root, validation_root
from pathlib import Path
import numpy as np, json

V = Path(str(validation_root()))
R = V / str(candidate_root())
A = dict(np.load(R / "full-fields.npz"))
F = np.load(R / "region.npz")["faces"]
B = dict(np.load(V / "asy-proposal-pairs-joint-rebuild-20261112/full-fields.npz"))
T = np.load(V / "asy-proposal-pairs-joint-rebuild-20261112/full-geometry.npz")["faces"]
src = [
    x["face"]
    for x in json.loads(
        (V / "atd-band-protected-constraint-audit-20261112/SCOPE.json").read_text()
    )
]
inv = np.linalg.inv(
    np.array([np.vstack([A["TEXCOORD_1"][t].T, np.ones(3)]) for t in F])
)
rows = []
for f in src:
    for bc in [
        [1 / 3] * 3,
        [0.6, 0.2, 0.2],
        [0.2, 0.6, 0.2],
        [0.2, 0.2, 0.6],
        [0.5, 0.5, 0],
        [0, 0.5, 0.5],
        [0.5, 0, 0.5],
    ]:
        bc = np.array(bc)
        q = bc @ B["TEXCOORD_1"][T[f]]
        ws = inv @ np.r_[q, 1]
        js = np.where(ws.min(1) > -1e-08)[0]
        if not len(js):
            continue
        j = js[0]
        w = ws[j]
        d = w @ A["morph0.POSITION"][F[j]] - bc @ B["morph0.POSITION"][T[f]]
        rows.append(
            dict(
                source_face=f,
                candidate_face=int(j),
                triangle=F[j].tolist(),
                bary=bc.tolist(),
                component_mm=float(np.max(abs(d)) * 1000),
                norm_mm=float(np.linalg.norm(d) * 1000),
            )
        )
target = [r for r in rows if r["source_face"] in [5407, 5408, 5412]]
(R / "TARGET-MOUTHOPEN.json").write_text(json.dumps(target, indent=2))
print("targetmax", max((r["component_mm"] for r in target)))
rows.sort(key=lambda x: -x["component_mm"])
(R / "MOUTHOPEN-ERROR-ORIGIN.json").write_text(json.dumps(rows[:12], indent=2))
print(rows[:4])
