"""Opt-in lower-medial normal continuity; no geometry or region mixing."""
import math


def validate(parameters):
    if parameters is None:
        return 0.0
    if not isinstance(parameters, dict) or set(parameters) - {"lower_medial_blend"}:
        raise ValueError("Unknown normal parameter")
    amount = parameters.get("lower_medial_blend", 0.0)
    if (
        isinstance(amount, bool)
        or not isinstance(amount, (int, float))
        or not math.isfinite(amount)
        or not 0 <= amount <= 1
    ):
        raise ValueError("lower_medial_blend must be finite in [0, 1]")
    return float(amount)


def adapt_pipeline(source, amount):
    if amount == 0:
        return source

    def replace(old, new):
        nonlocal source
        if source.count(old) != 1:
            raise ValueError("Normal source contract changed: " + old[:70])
        source = source.replace(old, new, 1)

    replace("        region_normals = {}", "        region_normals = {}\n        unsplit_skin_normals = {}")
    replace(
        '                normals[index] += poly.normal * poly.area',
        '                normals[index] += poly.normal * poly.area\n'
        '                if region in (1, 2):\n'
        '                    skin_key = (index, region)\n'
        '                    unsplit_skin_normals.setdefault(skin_key, Vector((0, 0, 0)))\n'
        '                    unsplit_skin_normals[skin_key] += poly.normal * poly.area',
    )
    replace(
        '        for normal in normals:\n',
        '        for normal in unsplit_skin_normals.values():\n'
        '            normal.normalize()\n'
        '        for normal in normals:\n',
    )
    anchor = '                if index in ear_root_ids:\n                    normal = normal.lerp'
    insert = '''                if poly.use_smooth and region in (1, 2) and side != 0:
                    from facial_surface_runtime import lower_medial_normal_weight
                    h = (data.vertices[index].co.z + 0.021 - C["origin_y_m"]) / C["height_m"]
                    weight = lower_medial_normal_weight(h, LOWER_MEDIAL_AMOUNT)
                    if weight > 0:
                        target = unsplit_skin_normals[(index, region)]
                        assert target.length > 1e-12, "Degenerate lower-medial normal"
                        normal = normal.lerp(target, weight).normalized()
'''.replace("LOWER_MEDIAL_AMOUNT", repr(amount))
    replace(anchor, insert + anchor)
    return source
