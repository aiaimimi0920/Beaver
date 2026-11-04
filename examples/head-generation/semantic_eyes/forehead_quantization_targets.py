"""Stronger working targets for the original float32 saved-state gates."""
import numpy as np
from orbital_support_rule import orbital_quad_quality
from forehead_lp_restore import forehead_quantization_margin_targets
from forehead_orientation_margin import (forehead_signed_chart_cross_mm2,
                                         forehead_orientation_margin_targets)
from forehead_arc_quantization import forehead_arc_quantization_targets


def forehead_complete_margin_targets(smooth, saved, offset, scales, positive_scaled=1e-7):
    """Observed saved drift adds working clearance; it never changes admission.

    Inputs are the stage's original normalized residuals, including its energy
    row. Positive finite scales are the existing fixed raw-per-scaled vector.
    Only saved-failing owned rows receive targets. The caller retains their
    maximum across requests; every candidate still needs exact saved admission.
    """
    smooth, saved, scales = (np.asarray(v, float) for v in (smooth, saved, scales))
    if (smooth.ndim != 1 or not len(smooth) or saved.shape != smooth.shape
            or scales.ndim != 1 or type(offset) is not int
            or offset < 0 or offset + len(smooth) > len(scales)
            or not np.isfinite(np.r_[smooth, saved, scales]).all()
            or np.any(scales <= 0) or isinstance(positive_scaled, (bool, np.bool_))
            or not isinstance(positive_scaled, (int, float, np.integer, np.floating))
            or not np.isfinite(positive_scaled)
            or positive_scaled <= 0):
        raise ValueError('Invalid complete-guide quantization target ownership/scaling')
    targets = np.zeros(len(scales))
    failed = np.flatnonzero(saved < 0)
    if not len(failed):
        return targets
    scale = scales[offset:offset + len(smooth)][failed]
    with np.errstate(over='ignore', under='ignore', invalid='ignore'):
        current, rounded = smooth[failed] / scale, saved[failed] / scale
        drift = np.maximum(smooth[failed] - saved[failed], 0) / scale
        roundoff = 32 * np.finfo(float).eps * np.maximum(1, np.maximum(abs(current), abs(rounded)))
        required = np.maximum(drift + np.maximum(positive_scaled, roundoff),
                              np.nextafter(current, np.inf))
    if not np.isfinite(required).all() or np.any(required < 0) or np.any(required <= current):
        raise ValueError('Unrepresentable complete-guide quantization clearance')
    with np.errstate(over='ignore', invalid='ignore'):
        targets[offset + failed] = required
    # Wider NumPy scalars can be finite before assignment but overflow the
    # float64 vector consumed by the solver. Validate the returned values too.
    if not np.isfinite(targets).all() or np.any(targets[offset + failed] <= current):
        raise ValueError('Unrepresentable complete-guide quantization clearance')
    return targets


def forehead_make_quantization_margins(evaluate, constraint_jacobian, layout, scales,
        quads, signs, chart_radius, accepted_cross, fairing_stage=None,
        flow_stage=None, complete_stage=None):
    """Retain existing target policies and include the complete-guide gate."""
    def quantization_margins(values, residuals):
        state = evaluate(values)
        if state is None:
            return None
        smooth = state[1]
        rounded, _ = orbital_quad_quality(state[0].astype(np.float32).astype(float), quads)
        lower_rows = np.arange(*layout['angle_lower'])
        upper_rows = np.arange(*layout['angle_upper'])
        jacobian = constraint_jacobian(values)
        targets, evidence = forehead_quantization_margin_targets(
            residuals, jacobian, scales, lower_rows, upper_rows, smooth, rounded)
        orientation_rows = np.arange(*layout['orientation'])
        rows = jacobian[orientation_rows]
        variable_rows = np.asarray(rows.multiply(rows).sum(axis=1)).ravel() > 1e-24
        smooth_cross = forehead_signed_chart_cross_mm2(
            state[0], quads, signs, chart_radius).ravel()
        rounded_cross = forehead_signed_chart_cross_mm2(
            state[0].astype(np.float32).astype(float), quads, signs, chart_radius).ravel()
        targets, orientation_evidence = forehead_orientation_margin_targets(
            targets, orientation_rows, scales, smooth_cross, rounded_cross,
            variable_rows, accepted_mm2=accepted_cross, interior_cap_mm2=.001)
        orientation_summary = {
            'working_target_row_count': len(orientation_evidence),
            'accepted_start_floor_cap_mm2': .001,
            'rounding_repairs': [row for row in orientation_evidence if row['rounding_failed']],
            'fixed_source_radius_m': chart_radius,
            'original_accepted_reference_retained': True}
        if fairing_stage:
            targets = np.maximum(targets, fairing_stage.quantization_targets(
                state[0], layout['whole_guide_fairing'][0], scales))
        arc_evidence = []
        if flow_stage:
            arc_targets, arc_evidence = forehead_arc_quantization_targets(
                state[0], flow_stage.spacing,
                layout['physical_flow'][0] + flow_stage.flow.count, scales)
            targets = np.maximum(targets, arc_targets)
        evidence = {'angles': evidence, 'orientation': orientation_summary,
                    'arc_quantization': arc_evidence}
        if complete_stage:
            complete_targets = complete_stage.quantization_targets(
                state[0], layout['complete_guide_fairing'][0], scales)
            targets = np.maximum(targets, complete_targets)
            evidence['complete_guide_quantization'] = {
                'rows': np.flatnonzero(complete_targets > 0).tolist(),
                'original_caps_and_saved_admission_unchanged': True}
        return targets, evidence

    return quantization_margins
