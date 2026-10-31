"""Persist complete native plans without emitting an unbound generation request."""
import json
from pathlib import Path
from .seed_evidence import finalize_seed_plan


def write_plan_files(plan, output):
    target = Path(output)
    if plan.get('source_seed') is None:
        paths = {'generation': target, 'source_packaging': target.with_suffix('.source-plan.json')}
        for key, path in paths.items():
            path.write_text(json.dumps(plan[key]))
        return paths
    paths = {'source_seed': target.with_suffix('.seed.json'), 'native_plan': target.with_suffix('.native-plan.json')}
    if any(path.exists() for path in [target, target.with_suffix('.source-plan.json'), *paths.values()]):
        raise ValueError('Seed preparation requires fresh request output paths')
    paths['source_seed'].write_text(json.dumps(plan['source_seed']))
    paths['native_plan'].write_text(json.dumps(plan))
    return paths


def finalize_plan_files(plan_path, receipt, workspace, output):
    plan = finalize_seed_plan(json.loads(Path(plan_path).read_text()), receipt, workspace)
    target = Path(output)
    paths = {'generation': target, 'source_packaging': target.with_suffix('.source-plan.json'),
             'finalized_plan': target.with_suffix('.finalized-plan.json')}
    if any(path.exists() for path in paths.values()):
        raise ValueError('Finalized requests require fresh output paths')
    for key in ('generation', 'source_packaging'):
        paths[key].write_text(json.dumps(plan[key]))
    paths['finalized_plan'].write_text(json.dumps(plan))
    return paths
