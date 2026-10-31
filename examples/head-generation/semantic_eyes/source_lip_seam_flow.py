"""Apply the authorized seam/cavity depth trial without changing source sampling."""
import inspect
from lip_seam_shift import shift_lip_seam


def adapt(sources):
    out = dict(sources)
    shell = out["shell.py"]
    hook = "            lip_depth_reports.append(refine_lip_depth(verts, rings, mouth_angles,\n                lip_depth_terms, depth, H, C))\n"
    for marker in ("def build_face():", hook, '        "joint_lip_depth": lip_depth_reports,'):
        if shell.count(marker) != 1:
            raise ValueError("Seam source contract changed: "+marker)
    shell = shell.replace("def build_face():",
                          inspect.getsource(shift_lip_seam)+"\n\ndef build_face():", 1)
    shell = shell.replace("    lip_depth_reports = []",
                          "    lip_depth_reports = []\n    lip_seam_reports = []", 1)
    shell = shell.replace(hook, hook+
                          "            lip_seam_reports.append(shift_lip_seam(verts, rings, mouth_angles, depth, H, C))\n", 1)
    shell = shell.replace('        "joint_lip_depth": lip_depth_reports,',
                          '        "joint_lip_depth": lip_depth_reports,\n        "coupled_lip_seam": lip_seam_reports,', 1)
    compile(shell, "shell.py", "exec")
    out["shell.py"] = shell
    return out
