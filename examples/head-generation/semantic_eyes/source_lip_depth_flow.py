"""Emit depth-only refinement into the existing bounded shell geometry module."""
import inspect
from lip_depth_solver import refine_lip_depth


def adapt(sources):
    out = dict(sources)
    shell = out['shell.py']
    build = 'def build_face():'
    hook = '                lambda x: Y0 + H*C["upper_lip_height"] + mouth_profile_shift(x)))\n'
    report = '        "joint_lip_support": lip_joint_reports,'
    for marker in (build, hook, report):
        if shell.count(marker) != 1:
            raise ValueError('Joint depth source contract changed: '+marker)
    shell = shell.replace(build, inspect.getsource(refine_lip_depth)+'\n\n'+build, 1)
    shell = shell.replace('    lip_joint_reports = []', '    lip_joint_reports = []\n    lip_depth_reports = []', 1)
    shell = shell.replace(hook, hook+
        '            lip_depth_reports.append(refine_lip_depth(verts, rings, mouth_angles,\n'
        '                lip_depth_terms, depth, H, C))\n', 1)
    shell = shell.replace(report, report+'\n        "joint_lip_depth": lip_depth_reports,', 1)
    compile(shell, 'shell.py', 'exec')
    out['shell.py'] = shell
    return out
