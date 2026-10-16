"""Opt-in derived-source fast path; immutable checkpoint and output math stay intact."""


def skip_zero_weight_sections(source):
    old_sections = """    sections = []
    for height, points in C["cross_sections"]:
        section_x = q * points[-1][0]
        sections.append((height, curve(section_x, 1, points)))
"""
    old_base = "    base = curve(v, 1, sections) * (1 - blend) + consistent * blend"
    if source.count(old_sections) != 1 or source.count(old_base) != 1:
        raise ValueError("Inherited base-depth fast-path contract changed")
    source = source.replace(old_sections, "", 1)
    new = """    if blend == 1:
        base = consistent
    else:
        sections = []
        for height, points in C["cross_sections"]:
            section_x = q * points[-1][0]
            sections.append((height, curve(section_x, 1, points)))
        base = curve(v, 1, sections) * (1 - blend) + consistent * blend"""
    return source.replace(old_base, new, 1)
