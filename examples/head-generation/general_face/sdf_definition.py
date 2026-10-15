"""Bounded-example retry adapter, appended to the unchanged upstream binder."""
_bind_directional_atlas_once = bind_directional_atlas


def bind_directional_atlas(source, atlas_path):
    atlas = list(re.finditer(r'(?m)^\[ext_resource type="Texture2D" path="[^"\r\n]+" id="sdf360_atlas"\]\r?\n?', source))
    binding = list(re.finditer(r'(?m)^face_sdf360 = ExtResource\("sdf360_atlas"\)\r?\n?', source))
    if not atlas and not binding and MATERIAL_SCRIPT not in source:
        return _bind_directional_atlas_once(source, atlas_path)
    if len(atlas) != 1 or len(binding) != 1 or source.count(MATERIAL_SCRIPT) != 1:
        raise ValueError("Incomplete or ambiguous existing directional atlas binding")
    original = source
    # Remove exactly the owned atlas declaration/property and restore the base
    # script before invoking the upstream validator. All unrelated text survives.
    for match in sorted(atlas + binding, key=lambda m: m.start(), reverse=True):
        source = source[:match.start()] + source[match.end():]
    source = source.replace(MATERIAL_SCRIPT, BASE_SCRIPT, 1)
    _bind_directional_atlas_once(source, atlas_path)  # Validate shape and new path.
    declaration = atlas[0]
    replacement = re.sub(r'path="[^"\r\n]+"', lambda _: 'path="res://' + atlas_path + '"', declaration.group(0))
    return original[:declaration.start()] + replacement + original[declaration.end():]
