"""Prepare the second managed request; never execute a model engine here."""

import argparse
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from source_layout.package import packaging_request


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-plan", type=Path, required=True)
    parser.add_argument("--generation-receipt", type=Path, required=True)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    target = args.output.resolve()
    if target.is_relative_to(args.workspace.resolve()) or target.is_relative_to(
        Path(__file__).resolve().parent.parent
    ):
        raise ValueError(
            "Request output must be outside the task and recipe directories"
        )
    request = packaging_request(
        json.loads(args.source_plan.read_text()),
        args.generation_receipt,
        args.workspace,
    )
    target.write_text(json.dumps(request))


if __name__ == "__main__":
    main()
