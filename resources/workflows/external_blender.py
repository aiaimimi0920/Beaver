"""Run authorized external-agent Blender Python in a Beaver-owned process.

This is trusted code execution, NOT an OS/Python sandbox. The staging helpers
prevent accidental undeclared publication; arbitrary Python still has the host
user's OS permissions. Never use this runner for untrusted code.
"""
import json
from pathlib import Path
import sys

import bpy


def main():
    directory = Path(sys.argv[sys.argv.index("--") + 1]).resolve(strict=True)
    context = json.loads((directory / "context.json").read_text(encoding="utf-8"))
    workspace = Path(context["workspace"]).resolve(strict=True)
    outputs = dict(context["outputs"])

    def beaver_output(relative):
        """Return the staging destination for an explicitly declared output."""
        if relative not in outputs:
            raise ValueError("Output was not declared: " + str(relative))
        return outputs[relative]

    def beaver_input(relative):
        """Resolve a regular workspace input without traversing linked paths."""
        if not isinstance(relative, str) or "\\" in relative or ":" in relative:
            raise ValueError("Invalid workspace-relative input")
        parts = relative.split("/")
        if any(not part or part.startswith(".") for part in parts):
            raise ValueError("Invalid workspace-relative input")
        path = workspace
        for part in parts:
            path = path / part
            if path.is_symlink() or (hasattr(path, "is_junction") and path.is_junction()):
                raise ValueError("Linked workspace input rejected")
        resolved = path.resolve(strict=True)
        if not resolved.is_relative_to(workspace) or not resolved.is_file():
            raise ValueError("Input must be a regular workspace file")
        return str(resolved)

    # Avoid undeclared .blend1 backup siblings in the staging area.
    bpy.context.preferences.filepaths.save_version = 0
    namespace = {
        "__name__": "__main__",
        "__file__": str(directory / "script.py"),
        "bpy": bpy,
        "BEAVER_WORKSPACE": str(workspace),
        "BEAVER_OUTPUTS": outputs,
        "beaver_output": beaver_output,
        "beaver_input": beaver_input,
    }
    code = (directory / "script.py").read_text(encoding="utf-8")
    exec(compile(code, str(directory / "script.py"), "exec"), namespace)
    (directory / "completed.json").write_text(
        json.dumps({"completed": True, "blenderVersion": bpy.app.version_string}),
        encoding="utf-8",
    )
    print("BEAVER_EXTERNAL_PYTHON_COMPLETED", flush=True)


if __name__ == "__main__":
    main()
