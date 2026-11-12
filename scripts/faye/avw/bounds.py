from paths import candidate_root, validation_root
from pathlib import Path
import numpy as np, json, itertools

V = Path(str(validation_root()))
R = V / str(candidate_root())
R.mkdir(exist_ok=True)
reports = []
for folder in [str(candidate_root())]:
    S = V / folder
    A = np.load(S / "full-fields.npz")
    F = np.load(S / "region.npz")["faces"]
    P = A["POSITION"][F]
    D = np.array([A[f"morph{k}.POSITION"][F] for k in range(7)])
    W = np.array(list(itertools.product([0, 0.5, 1], repeat=7)))
    q = P[None] + np.einsum("wm,mfvc->wfvc", W, D)
    cross = np.cross(q[:, :, 1] - q[:, :, 0], q[:, :, 2] - q[:, :, 0])
    normal = np.cross(P[:, 1] - P[:, 0], P[:, 2] - P[:, 0])
    normal /= np.linalg.norm(normal, axis=1)[:, None]
    signed = np.einsum("wfc,fc->wf", cross, normal)
    coef = signed.reshape((3,) * 7 + (len(F),)).copy()
    for axis in range(7):
        sl = [slice(None)] * 8
        sl[axis] = 1
        l = sl.copy()
        l[axis] = 0
        r = sl.copy()
        r[axis] = 2
        coef[tuple(sl)] = 2 * coef[tuple(sl)] - 0.5 * (coef[tuple(l)] + coef[tuple(r)])
    lower = coef.reshape(-1, len(F)).min(0)
    corners = np.array(list(itertools.product([0, 1], repeat=7)))
    z = P[None] + np.einsum("wm,mfvc->wfvc", corners, D)
    edge2 = np.sum((z - np.roll(z, 1, axis=2)) ** 2, axis=3).max(axis=(0, 2))
    upper = np.where(lower > 0, edge2 / lower, np.inf)
    obs = np.linalg.norm(q - np.roll(q, 1, axis=2), axis=3).max(
        2
    ) ** 2 / np.linalg.norm(cross, axis=2)
    rep = dict(
        input=folder,
        triangles=len(F),
        continuous_positive_projection_certificates=int(np.sum(lower > 0)),
        uncertified_faces=np.where(lower <= 0)[0].tolist(),
        min_projected_area_lower_mm2=float(lower.min() * 1000000.0),
        conservative_aspect_upper=float(upper.max()),
        sampled_aspect_max=float(obs.max()),
        worst_bound_face=int(upper.argmax()),
        worst_bound_vertices=F[upper.argmax()].tolist(),
        scope="exact polynomial Bernstein enclosure over independent seven weights in [0,1]; floating-point arithmetic, not formal interval rounding proof",
    )
    reports.append(rep)
    np.savez(
        R / "avw-bounds.npz",
        projected_area_lower=lower,
        edge_squared_upper=edge2,
        aspect_upper=upper,
    )
    print(rep)
(R / "CONTINUOUS-BOUNDS.json").write_text(json.dumps(reports, indent=2))
