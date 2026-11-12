from paths import candidate_root, validation_root
from pathlib import Path
import json, numpy as np

R = Path(str(candidate_root()))
F = np.load(R / "region.npz")["faces"]
(R / "ACTUAL-EDGES.json").write_text(
    json.dumps(
        sorted(
            {
                tuple(sorted((int(a), int(b))))
                for t in F
                for a, b in zip(t, np.roll(t, -1))
            }
        )
    )
)
print("edges updated")
