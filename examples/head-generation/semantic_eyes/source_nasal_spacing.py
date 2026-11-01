"""Install semantic five-rail density repair before independent chin refinement."""
import ast
from pathlib import Path
from source_nasal_flow import MODULES as NASAL_MODULES
from source_chin_retile import CHIN_MODULES

SPACING_MODULES = ('nasal_spacing_rule', 'nasal_rear_rule', 'nasal_return_provenance',
                   'orbital_support_rule', 'orbital_support_bind', 'orbital_support_native',
                   'mouth_corner_rule', 'mouth_corner_native', 'mouth_corner_shapes',
                   'forehead_flow_bind', 'forehead_flow_rule', 'forehead_flow_refine', 'forehead_flow_native',
                   'forehead_star_topology',
                   'forehead_boundary_route',
    'forehead_retile_roles',
                   'forehead_surface_batch',
                   'forehead_retile_fair',
    'forehead_fit_domain',
                   'forehead_retile_support',
    'forehead_affected_domain',
                   'forehead_retile_mirror',
                   'forehead_retile_contract',
                   'forehead_retile_fields',
    'forehead_source_fields',
                   'forehead_author_surface',
                   'forehead_author_pose_guard',
                   'forehead_ear_route',
                   'forehead_redistribute',
                   'forehead_fit_supports',
                   'forehead_surface_supports',
                   'forehead_solver_restore',
    'forehead_objective_jacobian',
    'forehead_fit_density',
    'forehead_spacing',
    'forehead_gap_measure',
    'forehead_physical_stage',
    'forehead_station_stage',
    'forehead_working_targets',
    'forehead_author_atlas',
    'forehead_author_gap_samples',
    'forehead_matched_gap_validation',
    'forehead_vertex_gap_jacobian',
    'forehead_matched_gap_stage',
    'forehead_matched_gap_jacobian',
    'forehead_hybrid_gap_jacobian',
    'forehead_pattern_catalogue',
    'forehead_pattern_solver',
    'forehead_pattern_aggregate_solver',
    'forehead_pattern_materialize',
    'forehead_pattern_initialization',
    'forehead_guide_fairing',
    'forehead_fairing_stage',
    'forehead_role_fairing',
    'forehead_fairing_guard',
    'forehead_fairing_solver',
    'forehead_fairing_restore',
    'forehead_lower_guard',
    'forehead_aspect_constraints',
    'forehead_hybrid_scaling',
    'forehead_anchor_domain',
    'forehead_seam_support',
    'forehead_failed_seam_support',
    'forehead_seam_hint',
    'forehead_lp_restore',
    'forehead_seam_restore',
    'forehead_seam_growth',
    'forehead_arc_quantization',
    'forehead_orientation_margin',
    'forehead_field_jacobian',
    'forehead_paired_export',
    'forehead_paired_stage',
    'forehead_shared_fit',
    'forehead_fairing_continuation',
    'forehead_flow_continuation',
    'forehead_shared_acceptance',
    'forehead_gap_continuation',
    'forehead_birth_geometry',
    'forehead_corridor_foundation',
    'forehead_corridor_seed',
    'forehead_corridor_setup',
    'forehead_initial_sector',
    'forehead_initial_construction',
    'forehead_initial_ready',
    'forehead_lower_rail_capacity',
    'forehead_lower_rail_recovery',
    'forehead_lower_rail_rebind',

    'forehead_shared_consumer',
                   'forehead_current_recipe',
                   'forehead_flow_acceptance',
                   'forehead_exact_distance',
    'source_polygon_validity',
    'forehead_support_contract',
    'forehead_support_band_selection',
    'forehead_support_band',
    'forehead_support_inset',
    'forehead_support_birth_operator',
    'forehead_support_witness_selection',
    'forehead_operator_shape_fit_samples',
    'forehead_operator_shape_analytic_samples',
    'forehead_operator_selected_shape_samples',
    'forehead_operator_reverse_shape_samples',
    'forehead_local_shape_operator',
                   'forehead_retile_shape',
                   'forehead_retile_apply',
                   'forehead_retile_mesh',
                   'nasal_spacing_shape',
                   'nasal_spacing_mesh', 'nasal_spacing_joint')


def adapt(sources):
    out = dict(sources)
    pipeline = out['pipeline.py']
    marker = '        from shell import refine_chin_source'
    prior = 'report.update(refine_nasal_source('
    if pipeline.count(marker) != 1 or pipeline.count(prior) != 1:
        raise ValueError('Five-rail spacing must follow nasal core and precede chin')
    if pipeline.index(prior) > pipeline.index(marker):
        raise ValueError('Five-rail spacing source stages changed order')
    out['pipeline.py'] = pipeline.replace(marker,
        '        from shell import refine_nasal_bridge_spacing, capture_nasal_spacing_baseline\n'
        '        _spacing_baseline = capture_nasal_spacing_baseline(data)\n'
        '        report.update(refine_nasal_bridge_spacing(data,semantic_part_ordinals,\n'
        '            report["nasal_inner_source"],\n'
        '            report["nasal_five_flow_source"]["allocated_flow_records"]))\n'
        '        report.update(refine_nasal_bridge_spacing(data,semantic_part_ordinals,\n'
        '            report["nasal_inner_source"], report["nasal_five_flow_source"]["allocated_flow_records"],\n'
        '            generated_return=report["ear_post_return"], baseline=_spacing_baseline))\n'
        '        report.update(refine_nasal_bridge_spacing(data,semantic_part_ordinals,\n'
        '            report["nasal_inner_source"], report["nasal_five_flow_source"]["allocated_flow_records"],\n'
        '            orbital_calibration=C, assembly_offset=report["head_assembly_y_offset_m"]))\n' + marker, 1)
    mouth_marker = '        report.update(refine_chin_source(data,report["chin_source_anchors"]))'
    if out['pipeline.py'].count(mouth_marker) != 1:
        raise ValueError('Mouth corner stage requires unique completed chin source transaction')
    out['pipeline.py'] = out['pipeline.py'].replace(mouth_marker, mouth_marker + '\n'
        '        report.update(refine_nasal_bridge_spacing(data,semantic_part_ordinals,\n'
        '            report["nasal_inner_source"], report["nasal_five_flow_source"]["allocated_flow_records"],\n'
        '            assembly_offset=report["head_assembly_y_offset_m"],\n'
        '            mouth_context={"calibration": C, "chin": report["chin_source_retile"]}))', 1)
    forehead_marker = '            mouth_context={"calibration": C, "chin": report["chin_source_retile"]}))'
    if out['pipeline.py'].count(forehead_marker) != 1:
        raise ValueError('Forehead stage requires unique completed mouth source stage')
    out['pipeline.py'] = out['pipeline.py'].replace(forehead_marker, forehead_marker + '\n'
        '        report.update(refine_nasal_bridge_spacing(data,semantic_part_ordinals,\n'
        '            report["nasal_inner_source"], report["nasal_five_flow_source"]["allocated_flow_records"],\n'
        '            assembly_offset=report["head_assembly_y_offset_m"],\n'
        '            forehead_context={"calibration": C, "report": report}))', 1)
    shell = out['shell.py']
    existing = {node.name for node in ast.parse(shell).body
                if isinstance(node, (ast.FunctionDef, ast.ClassDef))}
    internal = set(NASAL_MODULES) | set(CHIN_MODULES) | set(SPACING_MODULES)
    root = Path(__file__).parent
    chunks = []
    for module in SPACING_MODULES:
        source = (root / (module + '.py')).read_text(encoding='utf-8')
        excluded = set()
        for node in ast.parse(source).body:
            if isinstance(node, (ast.FunctionDef, ast.ClassDef)):
                if node.name in existing:
                    raise ValueError('Five-rail source helper collides: ' + node.name)
                existing.add(node.name)
            if isinstance(node, ast.ImportFrom) and node.module in internal:
                excluded.update(range(node.lineno - 1, node.end_lineno))
        chunks.append(''.join(line for i, line in enumerate(source.splitlines(keepends=True))
                              if i not in excluded))
    out['shell.py'] = shell + '\n\n' + '\n\n'.join(chunks)
    for name, source in out.items():
        if name.endswith('.py'):
            ast.parse(source, filename=name)
    return out
