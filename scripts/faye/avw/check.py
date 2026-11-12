from paths import candidate_root, validation_root
from pathlib import Path
import numpy as np, json

V = Path(str(validation_root()))
R = V / str(candidate_root())
A = dict(np.load(R / "full-fields.npz"))
B = dict(np.load(V / "asy-proposal-pairs-joint-rebuild-20261112/full-fields.npz"))
print(
    "source support vertex22max",
    {
        v: max((float(np.max(abs(A[k][v] - B[k][v]))) for k in B))
        for v in [7071, 7147, 7132, 7130]
    },
)
for f in ["finalaudit.log", "target.log", "trace.log"]:
    if (R / f).exists():
        print(f, (R / f).read_text()[:1000])
