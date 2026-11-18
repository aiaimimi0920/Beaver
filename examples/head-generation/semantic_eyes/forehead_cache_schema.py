FOREHEAD_CACHE_SCHEMAS={
'ForeheadCompleteCountedBatch':'stage batch role',
'ForeheadSurfaceBatch':'cache_limit candidate_cache xy vertices lower upper',
'ForeheadWorkingFlowStage':'active step_scale flow spacing state groups count',
'ForeheadPhysicalFlowStage':'paths step samples maximum_reduction goals caps front_caps front_goals groups count',
'ForeheadUniformStationStage':'paths goal reduction caps groups reference count',
'ForeheadRoleFairingStage':'paths reference triples initial_energy target_energy turn_caps turn_roles initial_metrics count nodes reduction bv_caps bv_norm role_caps',
'ForeheadCompleteGuideFairingStage':'source_identity config calls total_calls identity paths points_count faces frames quads nodes reference caps target count',
'ForeheadPairedDiagonalStage':'export_guard fields ids face_ids quads pose_rounding_offset triangles axes signs scales count groups floor',
'ForeheadPairedExportGuard':'faces face_ids fields',
'ForeheadAnchorDomain':'ids index fixed pivot specs margin initial threshold',
'ForeheadMatchedGapStage':'atlas fast paths protected count pairs targets groups labels verify_fast domains size target newly_covered_domains caps cap active',
'ForeheadFrozenAuthorAtlas':'author sc old refs names entry exit_h left right rulings evidence',
'ForeheadFrozenGapResidual':'cache_limit cache atlas paths names count entry height pivot roles endpoint_world endpoint_chart groups specs row_count',
}
FOREHEAD_CACHE_IGNORED={'ForeheadSurfaceBatch':{'candidate_cache'},'ForeheadFrozenGapResidual':{'cache'},'ForeheadCompleteGuideFairingStage':{'calls','total_calls'}}
FOREHEAD_CACHE_NAMES='old old_delta source_chart donors initial ids all_quads quads signs frozen_delta chart_radius original_params lower_guard lower_triples aspect_reference layout sparse scales batch delta_batch flow_stage fairing_stage complete_stage paired_stage paired_export anchor_domain gap_stage'.split()

FOREHEAD_CACHE_IGNORED_CLOSURES={
 ('fit_forehead_shared_constraints.<locals>.evaluate','cache'),
 ('fit_forehead_shared_constraints.<locals>.constraint_jacobian','field_jacobian_stats'),
 ('forehead_make_hybrid_gap_jacobian.<locals>.derivative','stats'),
 ('forehead_prepare_hybrid_scaling.<locals>.scaled_derivative','metadata'),
}
