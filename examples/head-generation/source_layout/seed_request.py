"""Bind a successful native source seed before emitting generation and packaging plans."""
import argparse
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from source_layout.plan_files import finalize_plan_files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--native-plan', type=Path, required=True)
    parser.add_argument('--seed-receipt', type=Path, required=True, action='append',
                        help='Repeat in exact seed-job order for schema3/4')
    parser.add_argument('--workspace', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    if output.is_relative_to(args.workspace.resolve()) or output.is_relative_to(Path(__file__).resolve().parent.parent):
        raise ValueError('Request output must be outside task and recipe directories')
    version = json.loads(args.native_plan.read_text())['source_seed_binding'].get('schema_version', 1)
    receipts = args.seed_receipt if version in (3, 4) else args.seed_receipt[0]
    if version not in (3, 4) and len(args.seed_receipt) != 1:raise ValueError('One receipt required')
    paths = finalize_plan_files(args.native_plan, receipts, args.workspace, output)
    for key, path in paths.items():
        print(f'{key}: {path}')


if __name__ == '__main__':
    main()
