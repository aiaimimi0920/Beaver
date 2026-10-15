"""Reject optional full-performance paths outside the bounded head-only contract."""
import re

FORBIDDEN_PROFILES = (
    "face_motion_profile", "face_atlas_profile", "eye_geometry_profile",
    "performance_data_path",
)


def require_head_only_definition(source):
    """Fail closed even for explicit null assignments; generated defaults omit them."""
    if not isinstance(source, str):
        raise ValueError("Expected textual Godot definition")
    for field in FORBIDDEN_PROFILES:
        if re.search(r"(?m)^\s*" + re.escape(field) + r"\s*=", source):
            raise ValueError("Bounded node-affine head example forbids optional profile: " + field)
    return source
