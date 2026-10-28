"""Split source-flow modules at explicit responsibility boundaries, preserving order."""
import ast
import unicodedata
from .layout import digest
from .python_lines import conservative_effective_lines

MODULE_STAGES = {
    'face.py': (
        ('face_surface_core.py', None),
        ('face_feature_parts.py', 'mesh'),
        ('face_source_sampling.py', 'density_rows'),
        ('face_source_corrections.py', 'orbital_endpoint_weight'),
    ),
    'shell.py': (
        ('shell_surface_geometry.py', None),
        ('shell_source_assembly.py', 'build_face'),
        ('shell_patch_sampling.py', 'capture_mouth_patch_fields'),
        ('shell_nasal_geometry.py', 'cross2'),
        ('shell_nasal_domain.py', 'connected_components'),
        ('shell_nasal_core.py', 'capture_nasal_source_anchors'),
        ('shell_nasal_binding.py', 'validate_domain'),
        ('shell_nasal_adaptation.py', 'adapt_nasal_patch_parameters'),
        ('shell_nasal_mesh.py', 'prepare_nasal_patch'),
        ('shell_nasal_flow_trace.py', 'trace_outer_flow'),
        ('shell_nasal_joint_contract.py', 'nasal_oriented_cycle'),
        ('shell_nasal_regularization.py', 'regularize_nasal_source'),
        ('shell_nasal_inner.py', 'propose_inner_nasal'),
        ('shell_nasal_inner_adaptation.py', 'inner_rail_triangle_nearest'),
        ('shell_nasal_inner_mesh.py', 'rebuild_inner_nasal_source'),
        ('shell_nasal_mesh_snapshot.py', 'nasal_mesh_snapshot'),
        ('shell_nasal_joint_mesh.py', 'refine_nasal_source'),
        ('shell_chin_roles.py', 'chin_oriented_cycle'),
        ('shell_chin_fields.py', 'chin_snapshot_point_fields'),
        ('shell_chin_contract.py', 'chin_graph'),
        ('shell_chin_template.py', 'chin_retile_recipe'),
        ('shell_chin_solver.py', 'chin_propose_retile'),
        ('shell_chin_shape.py', 'chin_shape_closest_triangles'),
        ('shell_chin_mesh.py', 'stage_chin_source_retile'),
        ('shell_chin_transaction.py', 'refine_chin_source'),
        ('shell_nasal_spacing_rule.py', 'propose_nasal_bridge_spacing'),
        ('shell_nasal_spacing_shape.py', 'guard_nasal_spacing_shape'),
        ('shell_nasal_spacing_mesh.py', 'stage_nasal_bridge_spacing'),
        ('shell_nasal_spacing_transaction.py', 'refine_nasal_bridge_spacing'),
    ),
    'ear_topology.py': (
        ('ear_source_legacy.py', None),
        ('ear_source_template.py', 'ear_quad_least_squares'),
        ('ear_source_return_caps.py', 'solve_return_cap'),
        ('ear_source_flow_geometry.py', 'source_flow_ear_layout'),
        ('ear_source_flow_assembly.py', 'flow_point_fields'),
    ),
}


def module_layout(source, entry_name):
    stages = MODULE_STAGES[entry_name]
    tree = ast.parse(source)
    if any(isinstance(n, ast.ImportFrom) and n.module == '__future__' for n in tree.body):
        raise ValueError('Future module semantics require an explicit migration')
    if any(isinstance(n, ast.Name) and unicodedata.normalize('NFKC', n.id).startswith('_source_layout_')
           for n in ast.walk(tree)):
        raise ValueError('Reserved source module frame name collision')
    functions = {}
    for node in tree.body:
        if isinstance(node, ast.FunctionDef):
            if node.name in functions:
                raise ValueError('Duplicate source function boundary: ' + node.name)
            functions[node.name] = node
    boundaries = [0]
    for _, name in stages[1:]:
        if name not in functions or functions[name].decorator_list:
            raise ValueError('Source module boundary changed: ' + name)
        boundaries.append(functions[name].lineno - 1)
    lines = source.splitlines(keepends=True)
    boundaries.append(len(lines))
    if boundaries != sorted(set(boundaries)):
        raise ValueError('Source module responsibilities changed order')
    chunks = {name: ''.join(lines[a:b]) for (name, _), a, b in zip(stages, boundaries, boundaries[1:])}
    if ''.join(chunks.values()) != source:
        raise AssertionError('Ordered module reconstruction changed')
    for name, text in chunks.items():
        compile(text, name, 'exec')
        effective = conservative_effective_lines(text)
        if effective > 500:
            raise ValueError('Source responsibility exceeds conservative limit: ' + name)
    doc = ast.get_docstring(tree, clean=False)
    entry = (repr(doc) + '\n') if doc is not None else ''
    entry += 'if "__file__" not in globals():\n    raise ValueError("Packaged source replay requires a filesystem-aware module frame")\n'
    entry += 'from pathlib import Path as _source_layout_Path\n'
    entry += f'_source_layout_names = {tuple(chunks)!r}\n'
    entry += 'for _source_layout_name in _source_layout_names:\n'
    entry += '    _source_layout_path = _source_layout_Path(__file__).with_name(_source_layout_name)\n'
    entry += '    exec(compile(_source_layout_path.read_text(encoding="utf-8"), str(_source_layout_path), "exec"), globals())\n'
    entry += 'del _source_layout_Path, _source_layout_names, _source_layout_name, _source_layout_path\n'
    compile(entry, entry_name, 'exec')
    record = {'kind': 'ordered_module', 'entry_name': entry_name,
              'original_sha256': digest(source), 'entry_sha256': digest(entry),
              'stages': [{'path': name, 'sha256': digest(text)} for name, text in chunks.items()]}
    return {entry_name: entry, **chunks}, record


def execute_ordered_module(entry_path, record, namespace):
    """Replay a validated packaged module in an explicitly filesystem-aware frame.

    The legacy checkpoint packager consumes full source, not these launchers.
    This post-packaging API executes the verified snapshot in the caller's
    ModuleType namespace; it never rereads a chunk after hash validation.
    """
    from pathlib import Path
    from .layout import reconstructed
    lexical = Path(entry_path).absolute()
    if lexical.is_symlink() or ".." in lexical.parts:
        raise ValueError("Linked or traversing module entry path")
    path = lexical.resolve()
    name = record.get('entry_name')
    if name not in MODULE_STAGES or path.name != name or record.get('kind') != 'ordered_module':
        raise ValueError('Unknown packaged module entry')
    expected = [stage for stage, _ in MODULE_STAGES[name]]
    if [item['path'] for item in record.get('stages', [])] != expected:
        raise ValueError('Unexpected packaged module stage paths')
    files = {}
    for filename in [name, *expected]:
        candidate = path.parent / filename
        if candidate.is_symlink() or not candidate.is_file():
            raise ValueError('Missing or linked packaged module source')
        files[filename] = candidate.read_text(encoding='utf-8')
    reconstructed(files, record)
    if '__file__' in namespace and Path(namespace['__file__']).resolve() != path:
        raise ValueError('Conflicting module frame path')
    namespace['__file__'] = str(path)
    for item in record['stages']:
        stage = item['path']
        exec(compile(files[stage], str(path.parent / stage), 'exec'), namespace)
