"""Run explicit corpus-free demonstrations, never an asset generation pipeline."""

import argparse
import json
from pathlib import Path

from .redistribute import redistribute


def synthetic_results():
    # Fixtures are imported only for this explicitly requested demonstration.
    from .tests.fixtures import grid, joint_grid

    results = []
    for factory, policy in (
        (grid, "RAIL_PROPOSAL_V1"),
        (joint_grid, "SYNTHETIC_JOINT_NONREGRESSION_V1"),
    ):
        for shape in ((4, 5), (5, 7)):
            source = factory(*shape)
            result = redistribute(source, source.readset(), policy=policy)
            results.append(
                {
                    "fixture": factory.__name__,
                    "shape": shape,
                    "source_sha256": source.sha256,
                    **result,
                }
            )
    return results


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--synthetic-demo", action="store_true", required=True)
    parser.add_argument("--output", type=Path, help="Write a new JSON file; default stdout")
    args = parser.parse_args(argv)
    output = json.dumps(synthetic_results(), indent=2, allow_nan=False) + "\n"
    if args.output is None:
        print(output, end="")
    else:
        # Do not silently replace prior evidence.
        with args.output.open("x", encoding="utf-8") as stream:
            stream.write(output)


if __name__ == "__main__":
    main()
