"""Emit joint support redistribution after the retained aperture correction."""
import inspect
from lip_patch_solver import refine_lip_patch


def adapt(sources):
    out = dict(sources)
    shell = out['shell.py']
    hook = '            lip_seam_reports.append(shift_lip_seam(verts, rings, mouth_angles, depth, H, C))\n'
    report = '        "coupled_lip_seam": lip_seam_reports,'
    for marker in ('def build_face():', hook, report, '    lip_seam_reports = []'):
        if shell.count(marker) != 1:
            raise ValueError('Joint lip patch source contract changed: '+marker)
    shell += '\n\n'+inspect.getsource(refine_lip_patch)+'\n'
    shell = shell.replace('    lip_seam_reports = []', '    lip_seam_reports = []\n    lip_patch_reports = []', 1)
    shell = shell.replace(hook, hook+
                          '            lip_patch_reports.append(refine_lip_patch(verts, rings, mouth_angles,\n'
                          '                lip_depth_terms, depth, H, C))\n', 1)
    shell = shell.replace(report, report+'\n        "joint_lip_patch": lip_patch_reports,', 1)
    compile(shell, 'shell.py', 'exec')
    out['shell.py'] = shell
    return out
