"""Verify an explicitly supplied batch; never launch jobs or extrapolate acceptance."""

import argparse
import json
from pathlib import Path
from compose import SLOTS
from verify_run import verify_run, NativeJobFailure


def verify_batch(specs, verifier=verify_run):
    rows = []
    for spec in specs:
        identity = {}
        try:
            identity = {k: spec[k] for k in ("selection", "fit_policy", "prefix")}
            if set(spec["selection"]) != set(SLOTS):
                raise ValueError(
                    "Each batch row needs exactly six component selections"
                )
            result = verifier(spec)
        except NativeJobFailure as error:
            result = {"status": error.status, "reason": str(error)}
        except FileNotFoundError as error:
            result = {"status": "incomplete", "reason": str(error)}
        except (ValueError, KeyError, TypeError) as error:
            result = {"status": "integrity_failed", "reason": str(error)}
        rows.append({**identity, **result})
    verified = [row for row in rows if row["status"] == "technically_verified"]
    pairs = {}
    for i, left in enumerate(SLOTS):
        for right in SLOTS[i + 1 :]:
            pairs[left + "/" + right] = sorted(
                {
                    (r["selection"][left], r["selection"][right], r["fit_policy"])
                    for r in verified
                }
            )
    return {
        "schema_version": 1,
        "runs": rows,
        "technically_verified_runs": len(verified),
        "observed_pairs": pairs,
        "scope": "Only supplied artifact sets verified. Every omitted combination remains untested. Observed pairs do not certify other full combinations or aesthetics.",
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--batch", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(verify_batch(json.loads(args.batch.read_text())), indent=2))
