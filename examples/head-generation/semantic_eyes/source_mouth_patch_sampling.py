"""Wire whole-patch sampling into the current source construction transaction."""
import inspect
from mouth_patch_geometry import (capture_mouth_patch_fields, mouth_patch_quad_weights,
                                 mouth_patch_feasible_step, mouth_patch_quad_quality)
from mouth_patch_sampling import redistribute_mouth_patch


def adapt(sources):
    out = dict(sources)
    shell = out['shell.py']
    capture = ('            lip_patch_reports.append(refine_lip_patch(verts, rings, mouth_angles,\n'
               '                lip_depth_terms, depth, H, C))\n')
    apply = '    joint_frame_report = resample_joint_frame(verts,levels,lateral,cols,joint_frame_surface_point)\n'
    report = '        "joint_outer_frame_flow": joint_frame_report,'
    for marker in (capture,apply,report,'    lip_patch_reports = []'):
        if shell.count(marker) != 1:
            raise ValueError('Whole mouth-patch source hook changed: '+marker)
    shell = shell.replace('    lip_patch_reports = []',
                          '    lip_patch_reports = []\n    mouth_patch_fields = []',1)
    shell = shell.replace(capture,capture+
                          '            mouth_patch_fields.append(capture_mouth_patch_fields(verts, rings, mouth_angles, H, C))\n',1)
    shell = shell.replace(apply,apply+
                          '    mouth_patch_sampling_report = redistribute_mouth_patch(verts, shell_faces[:front_count], mouth_patch_fields, levels, lateral, cols, lip_depth_terms)\n',1)
    shell = shell.replace(report,report+'\n        "whole_mouth_patch_sampling": mouth_patch_sampling_report,',1)
    functions = (capture_mouth_patch_fields,mouth_patch_quad_weights,mouth_patch_feasible_step,
                 mouth_patch_quad_quality,redistribute_mouth_patch)
    shell += '\n\n'+'\n\n'.join(inspect.getsource(fn) for fn in functions)+'\n'
    compile(shell,'shell.py','exec')
    out['shell.py'] = shell
    return out
