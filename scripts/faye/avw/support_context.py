from paths import candidate_root, validation_root
from pathlib import Path
import json, numpy as np, sys

V = Path(str(validation_root()))
S = V / "avd-joint-obsolete-crosslinks-20261112"
A = np.load(S / "full-fields.npz")
F = np.load(S / "region.npz")["faces"]
P = A["POSITION"]
U = A["TEXCOORD_1"]
B = np.load(V / "asy-proposal-pairs-joint-rebuild-20261112/full-fields.npz")
T = np.load(V / "asy-proposal-pairs-joint-rebuild-20261112/full-geometry.npz")["faces"]
src = [
    x["face"]
    for x in json.loads(
        (V / "atd-band-protected-constraint-audit-20261112/SCOPE.json").read_text()
    )
]
from shapely.geometry import LineString, Polygon
from shapely.ops import unary_union

arcs = json.loads(
    (V / "atc-current-arc-station-parameters-20261112/CURRENT-ARCS.json").read_text()
)
raw = arcs[0]["current_vertices"] + list(reversed(arcs[5]["current_vertices"]))
clean = []
for v in raw:
    if not clean or np.linalg.norm(U[v] - U[clean[-1]]) > 1e-10:
        clean.append(v)
band = Polygon(U[clean])
oldedges = {
    tuple(sorted((int(a), int(b)))) for t in T[src] for a, b in zip(t, np.roll(t, -1))
}
oldnet = unary_union([LineString(B["TEXCOORD_1"][list(e)]) for e in oldedges])
E = {tuple(sorted((int(a), int(b)))) for t in F for a, b in zip(t, np.roll(t, -1))}
report = []
colors = {}
oldbuffer = oldnet.buffer(1e-11)
sourcepolys = {f: Polygon(B["TEXCOORD_1"][T[f]]).buffer(-1e-11) for f in src}
outer = band.buffer(1e-11)
inner = band.buffer(-1e-11)
left = arcs[1]["current_vertices"]
right = arcs[2]["current_vertices"]
raw = left + list(reversed(right))
clean = []
for v in raw:
    if not clean or np.linalg.norm(U[v] - U[clean[-1]]) > 1e-12:
        clean.append(v)
band = Polygon(U[clean])
cur = unary_union([Polygon(U[t]) for t in F])
net = unary_union([LineString(U[list(e)]) for e in E]).buffer(1e-10)
from shapely.geometry import Point

top = [7048, 7227, 7226, 7225, 7224, 7223, 7234]
rp = [7234] + arcs[2]["current_vertices"][1:]
lp = arcs[1]["current_vertices"]
raw = top + rp[1:] + list(reversed(lp))
vv = []
for v in raw:
    if not vv or np.linalg.norm(U[v] - U[vv[-1]]) > 1e-12:
        vv.append(v)
D = Polygon(U[vv])
h = [1.543863223156921, 1.539267166001851, 1.5348688552893772]
rows = []
for y in h:
    pairs = []
    for path in [lp, rp]:
        hits = []
        for a, b in zip(path, path[1:]):
            ya, yb = (P[a, 1], P[b, 1])
            if min(ya, yb) <= y <= max(ya, yb) and abs(yb - ya) > 1e-12:
                t = float((y - ya) / (yb - ya))
                hits.append(
                    dict(edge=[a, b], t=t, uv=((1 - t) * U[a] + t * U[b]).tolist())
                )
        pairs.append(hits)
    rows.append(dict(y=y, pairs=pairs))
