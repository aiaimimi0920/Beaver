from paths import candidate_root, validation_root
from support_context import *

R = V / str(candidate_root())
R.mkdir(exist_ok=True)
import triangle
from shapely.ops import polygonize

A = dict(A)
out = {k: list(v) for k, v in A.items()}
uv = list(U)
lookup = {tuple(np.round(q, 12)): i for i, q in enumerate(U)}
pr = []


def node(q):
    key = tuple(np.round(q, 12))
    if key in lookup:
        return lookup[key]
    for t in F:
        w = np.linalg.solve(np.vstack([U[t].T, np.ones(3)]), np.r_[q, 1])
        if w.min() > -1e-08:
            break
    else:
        raise RuntimeError("outside surface")
    v = len(uv)
    uv.append(q)
    lookup[key] = v
    for k in out:
        out[k].append(w @ A[k][t])
    pr.append(dict(vertex=v, current_triangle=t.tolist(), bary=w.tolist()))
    return v


lines = [D.boundary]
grid = []
for row in rows:
    assert all((len(x) == 1 for x in row["pairs"]))
    a, b = [np.array(x[0]["uv"]) for x in row["pairs"]]
    if len(grid) == 1:
        hh = P[7147, 1]
        hits = []
        for aa, bb in zip(lp, lp[1:]):
            ya, yb = (P[aa, 1], P[bb, 1])
            if min(ya, yb) <= hh <= max(ya, yb) and abs(ya - yb) > 1e-12:
                tt = (hh - ya) / (yb - ya)
                hits.append((1 - tt) * U[aa] + tt * U[bb])
        assert len(hits) == 1
        a = hits[0]
        c = U[7147]
        b = U[7132]
        q = [a, (a + c) / 2, c, (c + b) / 2, b]
    else:
        q = [(1 - s) * a + s * b for s in [0, 0.25, 0.5, 0.75, 1]]
    if len(grid) == 2:
        endpoints = []
        for path, hh in [(lp, P[7150, 1]), (rp, P[7083, 1])]:
            hits = []
            for aa, bb in zip(path, path[1:]):
                ya, yb = (P[aa, 1], P[bb, 1])
                if min(ya, yb) <= hh <= max(ya, yb) and abs(ya - yb) > 1e-12:
                    tt = (hh - ya) / (yb - ya)
                    hits.append((1 - tt) * U[aa] + tt * U[bb])
            assert len(hits) == 1
            endpoints.append(hits[0])
        aa, bb = endpoints
        support = [aa, (aa + U[7150]) / 2, U[7150], U[7083], bb]
        grid.append(support)
        lines.append(LineString(support))
    grid.append(q)
    lines.append(LineString(q))
g = np.array(grid)
for j, t in enumerate([0.1, 0.29164014306187636, 0.5220042979397891], 1):
    g[0, j] = (1 - t) * g[0, 0] + t * g[0, 4]
inv = np.linalg.inv(np.array([np.vstack([U[t].T, np.ones(3)]) for t in F]))


def phys(q):
    w = inv @ np.r_[q, 1]
    idx = np.where(w.min(1) > -1e-08)[0]
    if not len(idx):
        raise ValueError("outside")
    t = F[idx[0]]
    w = w[idx[0]]
    return np.array(
        [
            w
            @ (
                A["POSITION"][t]
                + sum(
                    (
                        weight * A["morph%d.POSITION" % k][t]
                        for k, weight in enumerate(ws)
                    ),
                    np.zeros((3, 3)),
                )
            )
            for ws in [
                [0] * 7,
                [1, 0, 0, 0, 0, 0, 0],
                [1, 0, 0, 0, 1, 0, 0],
                [0, 0, 0, 1, 0, 1, 0],
            ]
        ]
    )


def endpoints(h):
    result = []
    for path in [lp, rp]:
        hits = []
        for aa, bb in zip(path, path[1:]):
            ya, yb = (P[aa, 1], P[bb, 1])
            if min(ya, yb) <= h <= max(ya, yb) and abs(ya - yb) > 1e-12:
                t = (h - ya) / (yb - ya)
                hits.append((1 - t) * U[aa] + t * U[bb])
        if len(hits) != 1:
            raise ValueError("ambiguous")
        result.append(hits[0])
    return result


upper = np.array([phys(q) for q in g[2]])
records = []
for hh in np.linspace(1.533, 1.5362, 17):
    aa, bb = endpoints(hh)
    for ts in [
        [0.25, 0.5, 0.75],
        [0.3, 0.6, 0.82],
        [0.35, 0.65, 0.85],
        [0.4, 0.7, 0.9],
        [0.408631058638, 0.8, 0.9],
    ]:
        q = np.array([(1 - t) * aa + t * bb for t in [0] + ts + [1]])
        lo = np.array([phys(x) for x in q])
        scores = []
        for pose in range(4):
            for j in range(4):
                width = (
                    np.linalg.norm(upper[j + 1, pose] - upper[j, pose])
                    + np.linalg.norm(lo[j + 1, pose] - lo[j, pose])
                ) / 2
                length = (
                    np.linalg.norm(lo[j, pose] - upper[j, pose])
                    + np.linalg.norm(lo[j + 1, pose] - upper[j + 1, pose])
                ) / 2
                scores.append(abs(np.log(max(width, 1e-09) / max(length, 1e-09))))
        score = max(scores) + 0.25 * np.mean(scores)
        records.append(
            dict(height=float(hh), stations=ts, score=float(score), grid=q.tolist())
        )
best = min(records, key=lambda r: r["score"])
g[3] = best["grid"]
grid = list(g)
(R / "WIDTH-SEARCH.json").write_text(json.dumps(dict(best=best, trials=records)))
lines = [D.boundary]
for r, indices in [
    (0, [0, 1, 2, 4]),
    (1, [0, 1, 2, 4]),
    (2, [0, 2, 3, 4]),
    (3, [0, 2, 3, 4]),
]:
    lines.append(LineString([grid[r][j] for j in indices]))
lines.append(LineString([grid[0][1], grid[1][1]]))
lines.append(LineString([grid[0][2], U[7071], grid[1][2]]))
lines.append(LineString([U[7071], U[7132]]))
lines.append(LineString([U[7071], U[7130]]))
lines.append(LineString([grid[1][1], grid[2][2]]))
lines.append(LineString([grid[1][2], grid[2][3]]))
lines.append(LineString([grid[1][4], grid[2][3]]))
for j in [2, 3]:
    lines.append(LineString([grid[2][j], grid[3][j]]))
protected = [5445, 5448, 5449, 5451, 5454, 5428, 5452, 5453, 5456, 5455, 5436]
protect = unary_union([Polygon(B["TEXCOORD_1"][T[f]]) for f in protected])
patches = {
    i: Polygon(U[t]).intersection(protect).intersection(D) for i, t in enumerate(F)
}
patches = {i: z for i, z in patches.items() if z.area > 1e-16}
parent = {i: i for i in patches}


def root(i):
    while parent[i] != i:
        parent[i] = parent[parent[i]]
        i = parent[i]
    return i


edgefaces = {}
for i in patches:
    for a, b in zip(F[i], np.roll(F[i], -1)):
        key = tuple(sorted((tuple(np.round(U[a], 12)), tuple(np.round(U[b], 12)))))
        edgefaces.setdefault(key, []).append(i)
audit = []
for edge, ids in edgefaces.items():
    if len(ids) != 2:
        continue
    i, j = ids
    shared = patches[i].intersection(patches[j])
    length = shared.length
    if length < 1e-12:
        continue
    errors = {k: 0.0 for k in A}
    for x, y in [(i, j), (j, i)]:
        w = np.linalg.solve(
            np.vstack([U[F[x]].T, np.ones(3)]), np.vstack([U[F[y]].T, np.ones(3)])
        )
        for k in A:
            errors[k] = max(
                errors[k], float(np.max(abs(w.T @ A[k][F[x]] - A[k][F[y]])))
            )
    ok = max(errors.values()) < 1e-10
    audit.append(
        dict(
            faces=[int(i), int(j)], shared_length=float(length), merge=ok, errors=errors
        )
    )
    if ok:
        parent[root(j)] = root(i)
groups = {}
for i, z in patches.items():
    groups.setdefault(root(i), []).append(z)
for zs in groups.values():
    lines.append(unary_union(zs).boundary)
(R / "PARTITION-MERGE-AUDIT.json").write_text(
    json.dumps(dict(patches=len(patches), groups=len(groups), edges=audit))
)
segments = []
active_before = set(map(int, np.unique(F)))


def canonical_line(obj):
    if obj.geom_type in ("LineString", "LinearRing"):
        cs = list(obj.coords)
        for aa, bb in zip(cs, cs[1:]):
            aa = np.array(aa)
            bb = np.array(bb)
            d = bb - aa
            dd = d @ d
            if dd < 1e-28:
                continue
            ts = (U - aa) @ d / dd
            ds = np.linalg.norm(U - aa - ts[:, None] * d, axis=1)
            hits = [(0.0, aa), (1.0, bb)]
            hits += [
                (float(t), U[i])
                for i, t in enumerate(ts)
                if i in active_before and -1e-10 <= t <= 1 + 1e-10 and (ds[i] < 1e-11)
            ]
            hits.sort(key=lambda x: x[0])
            ss = []
            for t, q in hits:
                if ss and abs(t - ss[-1][0]) < 1e-10:
                    ss[-1] = (t, q)
                else:
                    ss.append((t, q))
            segments.extend(
                (
                    LineString([a[1], b[1]])
                    for a, b in zip(ss, ss[1:])
                    if np.linalg.norm(a[1] - b[1]) > 1e-12
                )
            )
    elif hasattr(obj, "geoms"):
        for x in obj.geoms:
            canonical_line(x)


for l in lines:
    canonical_line(l)
net = unary_union(segments)
cells = [p for p in polygonize(net) if D.buffer(1e-11).covers(p.representative_point())]
pieces = list(cells)
affected = []
for i, t in enumerate(F):
    p = Polygon(U[t])
    if p.intersection(D).area > 1e-16:
        affected.append(i)
        z = p.difference(D)
        if z.geom_type == "Polygon" and z.area > 1e-16:
            pieces.append(z)
        elif hasattr(z, "geoms"):
            pieces.extend(
                (x for x in z.geoms if x.geom_type == "Polygon" and x.area > 1e-16)
            )
for p in pieces:
    for q in list(p.exterior.coords)[:-1]:
        node(np.array(q))
allowed = set(map(int, np.unique(F)))
for pp in pieces:
    for qq in list(pp.exterior.coords)[:-1]:
        allowed.add(node(np.array(qq)))
new = []
preflight = []
for p in pieces:
    vs = []
    xy = np.array(uv)
    for qa, qb in zip(list(p.exterior.coords), list(p.exterior.coords)[1:]):
        qa = np.array(qa)
        d = np.array(qb) - qa
        den = d @ d
        if den < 1e-28:
            continue
        tt = (xy - qa) @ d / den
        dist = np.linalg.norm(xy - qa - tt[:, None] * d, axis=1)
        vv = np.where((tt >= -1e-09) & (tt < 1 - 1e-09) & (dist < 1e-11))[0]
        for v in sorted([v for v in vv if int(v) in allowed], key=lambda v: tt[v]):
            if not vs or np.linalg.norm(xy[v] - xy[vs[-1]]) > 1e-12:
                vs.append(int(v))
    if len(vs) < 3:
        continue
    xx = xy[vs]
    pp = np.array(out["POSITION"])[vs]
    jac = np.linalg.lstsq(np.c_[xx - xx.mean(0), np.ones(len(xx))], pp, rcond=None)[0][
        :2
    ]
    ev, vec = np.linalg.eigh(jac @ jac.T)
    metric = vec @ np.diag(np.sqrt(np.maximum(ev, 1e-20))) @ vec.T
    tr = triangle.triangulate(
        dict(
            vertices=(xx - xx.mean(0)) @ metric,
            segments=np.array([[i, (i + 1) % len(vs)] for i in range(len(vs))]),
        ),
        "pQ",
    )
    assert len(tr["vertices"]) == len(vs)
    new.extend(np.array(vs)[tr["triangles"]])
new = np.array(new)
out = {k: np.array(v) for k, v in out.items()}
p = out["POSITION"]
n = np.cross(p[new[:, 1]] - p[new[:, 0]], p[new[:, 2]] - p[new[:, 0]])
flip = (n * out["NORMAL"][new].mean(1)).sum(1) < 0
new[flip] = new[flip][:, [0, 2, 1]]
reg = np.vstack([np.delete(F, affected, axis=0), new])
full = np.load(S / "full-geometry.npz")["faces"]
removed = {tuple(sorted(t)) for t in F[affected]}
full = np.vstack([[t for t in full if tuple(sorted(t)) not in removed], new])
np.savez(R / "full-fields.npz", **out)
np.savez(R / "region.npz", faces=reg, uv=np.array(uv))
np.savez(R / "full-geometry.npz", faces=full)
(R / "ACTUAL-EDGES.json").write_text(
    json.dumps(
        sorted(
            {
                tuple(sorted((int(a), int(b))))
                for t in reg
                for a, b in zip(t, np.roll(t, -1))
            }
        )
    )
)
(R / "PROVENANCE.json").write_text(json.dumps(pr))
rep = dict(
    old_faces=len(affected),
    new_faces=len(new),
    region_faces=len(reg),
    newrows=len(pr),
    coverage=unary_union([Polygon(np.array(uv)[t]) for t in new])
    .symmetric_difference(unary_union([Polygon(U[t]) for t in F[affected]]))
    .area,
)
(R / "BUILD.json").write_text(json.dumps(rep))
print(rep)
