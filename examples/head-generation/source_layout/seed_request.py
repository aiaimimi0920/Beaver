"""Bind a successful native source seed before emitting generation and packaging plans."""
import argparse
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from source_layout.plan_files import finalize_plan_files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native-plan', type=Path, required=True)
    parser.add_argument('--seed-receipt', type=Path, required=True)
    parser.add_argument('--workspace', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    if output.is_relative_to(args.workspace.resolve()) or output.is_relative_to(Path(__file__).resolve().parent.parent):
        raise ValueError('Request output must be outside task and recipe directories')
    paths = finalize_plan_files(args.native_plan, args.seed_receipt, args.workspace, output)
    for key, path in paths.items():
        print(f'{key}: {path}')


if __name__ == '__main__':
    main()
