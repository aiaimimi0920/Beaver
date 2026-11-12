from paths import candidate_root, validation_root
from pathlib import Path
import numpy as np, json, collections

V = Path(str(validation_root()))
R = V / str(candidate_root())
R.mkdir(exist_ok=True)
D = json.loads(
    (V / "avw-source5407-interior-support-20261112/DARK-MARK-RAYS.json").read_text()
)
report = {}
allrows = {}
center = np.array([-90.0, 80.0, 140.0])
axis = np.array([0.0, -15.0, 0.0]) - center
axis /= np.linalg.norm(axis)
ex = np.cross(axis, [0.0, 1.0, 0.0])
ex /= np.linalg.norm(ex)
ey = np.cross(axis, ex)
lights = np.array(
    [center]
    + [
        center + rho * (np.cos(t) * ex + np.sin(t) * ey)
        for rho in [20, 39]
        for t in np.linspace(0, 2 * np.pi, 12, endpoint=False)
    ]
)
for name, ap, gp in [
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
    A = np.load(ap)
    F = np.load(gp)["faces"]
    P = (
        A["POSITION"]
        + A["morph0.POSITION"]
        - np.array([0, 1.5581640002441406, 0.0863528327703476])
    ) / 0.000708
    tr = P[F]
    e1 = tr[:, 1] - tr[:, 0]
    e2 = tr[:, 2] - tr[:, 0]
    rows = []
    cnt = collections.Counter()
    for r in D[name]:
        px, py = r["pixel"]
        if not (430 <= px <= 480 and 440 <= py <= 449):
            continue
        pos = np.array(
            [(px + 0.5 - 400) / 800 * 165, -15 + (350 - py - 0.5) / 800 * 165, r["z"]]
        )
        hits = []
        for k, L in enumerate(lights):
            dd = L - pos
            length = np.linalg.norm(dd)
            dd /= length
            h = np.cross(dd, e2)
            det = np.einsum("ij,ij->i", e1, h)
            inv = np.divide(1.0, det, out=np.zeros_like(det), where=abs(det) > 1e-12)
            s = pos - tr[:, 0]
            u = np.einsum("ij,ij->i", s, h) * inv
            q = np.cross(s, e1)
            v = q @ dd * inv
            t = np.einsum("ij,ij->i", e2, q) * inv
            ok = (
                (abs(det) > 1e-12)
                & (u >= -1e-10)
                & (v >= -1e-10)
                & (u + v <= 1 + 1e-10)
                & (t > 1e-05)
                & (t < length)
            )
            ok[r["face"]] = False
            idx = np.flatnonzero(ok)
            if len(idx):
                j = int(idx[np.argmin(t[idx])])
                hits.append(
                    dict(
                        light=k,
                        face=j,
                        distance_model=float(t[j]),
                        vertices=F[j].tolist(),
                    )
                )
                cnt[j] += 1
        rows.append(
            dict(pixel=r["pixel"], surface_face=r["face"], blocked=len(hits), hits=hits)
        )
    allrows[name] = rows
    report[name] = dict(
        pixels=len(rows),
        blocked_pixels=sum((x["blocked"] > 0 for x in rows)),
        blocked_rays=sum((x["blocked"] for x in rows)),
        occluders=cnt.most_common(20),
    )
src = {tuple(x["pixel"]): x for x in allrows["source"]}
diff = [
    dict(
        pixel=r["pixel"],
        source=src[tuple(r["pixel"])]["blocked"],
        trial=r["blocked"],
        extra=r["blocked"] - src[tuple(r["pixel"])]["blocked"],
        hits=r["hits"],
    )
    for r in allrows["trial"]
]
diff.sort(key=lambda x: x["extra"], reverse=True)
report["largest_extra"] = diff[:10]
(R / "RAYS.json").write_text(json.dumps(allrows))
(R / "SHADOW-SUMMARY.json").write_text(json.dumps(report, indent=2))
print({k: val for k, val in report.items() if k != "largest_extra"})
