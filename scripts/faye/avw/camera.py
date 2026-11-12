from paths import candidate_root, validation_root
from pathlib import Path
import numpy as np, json

V = Path(str(validation_root()))
R = V / str(candidate_root())
data = {}
for name, fieldpath, geompath in [
    (
        "source",
        V / "apk-coverage-correction-20261112/fields.npz",
        V / "apk-coverage-correction-20261112/new.npz",
    ),
    (
        "trial",
        V / "avw-source5407-interior-support-20261112/full-fields.npz",
        V / "avw-source5407-interior-support-20261112/full-geometry.npz",
    ),
]:
    A = np.load(fieldpath)
    F = np.load(geompath)["faces"]
    P = (
        A["POSITION"]
        + A["morph0.POSITION"]
        - np.array([0, 1.5581640002441406, 0.0863528327703476])
    ) / 0.000708
    norm = A["NORMAL"] + A["morph0.NORMAL"]
    tr = P[F]
    ab = tr[:, 1, :2] - tr[:, 0, :2]
    ac = tr[:, 2, :2] - tr[:, 0, :2]
    den = ab[:, 0] * ac[:, 1] - ab[:, 1] * ac[:, 0]
    valid = abs(den) > 1e-12
    den = np.where(valid, den, 1)
    out = []
    for py in range(435, 456):
        for px in range(420, 491, 2):
            xy = np.array(
                [(px + 0.5 - 400) / 800 * 165, -15 + (350 - py - 0.5) / 800 * 165]
            )
            d = xy - tr[:, 0, :2]
            b = (d[:, 0] * ac[:, 1] - d[:, 1] * ac[:, 0]) / den
            c = (ab[:, 0] * d[:, 1] - ab[:, 1] * d[:, 0]) / den
            a = 1 - b - c
            hit = np.where(valid & (a >= 0) & (b >= 0) & (c >= 0))[0]
            if not len(hit):
                continue
            z = a[hit] * tr[hit, 0, 2] + b[hit] * tr[hit, 1, 2] + c[hit] * tr[hit, 2, 2]
            j = int(hit[z.argmax()])
            w = np.array([a[j], b[j], c[j]])
            nn = w @ norm[F[j]]
            nn /= np.linalg.norm(nn)
            pos = w @ tr[j]
            light = np.array([-90, 80, 140]) - pos
            light /= np.linalg.norm(light)
            out.append(
                dict(
                    pixel=[px, py],
                    face=j,
                    z=float(pos[2]),
                    normal=nn.tolist(),
                    key_dot=float(nn @ light),
                    front_normal_z=float(
                        np.cross(tr[j, 1] - tr[j, 0], tr[j, 2] - tr[j, 0])[2]
                    ),
                )
            )
    data[name] = out
(R / "DARK-MARK-RAYS.json").write_text(json.dumps(data))
rows = []
for name, rr in data.items():
    for y in range(435, 456):
        r = [x for x in rr if x["pixel"][1] == y]
        rows.append(
            dict(
                mesh=name,
                y=y,
                min_dot=min((x["key_dot"] for x in r)),
                max_dot=max((x["key_dot"] for x in r)),
                faces=sorted(set((x["face"] for x in r))),
                min_z=min((x["z"] for x in r)),
                max_z=max((x["z"] for x in r)),
            )
        )
(R / "RAY-ROW-SUMMARY.json").write_text(json.dumps(rows, indent=2))
print([x for x in rows if x["y"] in [440, 443, 444, 445, 446, 449]])
