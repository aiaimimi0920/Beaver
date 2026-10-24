"""Original closed thin-ribbon primitives shared by eye accents."""
from face import *


def outline_sections(outline):
    """Cross-sections of an x-monotone semantic outline, including pointed ends."""
    result = []
    edges = list(zip(outline, outline[1:] + outline[:1]))
    controls = sorted({p[0] for p in outline})
    stations = set(controls)
    for a,b in zip(controls,controls[1:]):
        count = max(1,int((b-a)/STYLE["accent_max_station_gap"])+1)
        stations.update(a+(b-a)*i/count for i in range(1,count))
    for x in sorted(stations):
        hits = []
        for a, b in edges:
            if abs(a[0] - b[0]) < 1e-10:
                if abs(x - a[0]) < 1e-10:
                    hits.extend([a[1], b[1]])
            elif min(a[0], b[0]) - 1e-10 <= x <= max(a[0], b[0]) + 1e-10:
                f = (x - a[0]) / (b[0] - a[0])
                hits.append(a[1] + f * (b[1] - a[1]))
        assert hits
        result.append((x, min(hits), max(hits)))
    return result


def accent_sections(key):
    if not key.endswith("_rails"):
        return outline_sections(STYLE[key])
    rails = STYLE[key]
    if key == "upper_lash_rails" and STYLE.get("sampling_main_exact", False):
        return [tuple(row) for row in rails]
    stations = set(p[0] for p in rails)
    for a,b in zip(rails,rails[1:]):
        n = max(1,int((b[0]-a[0])/STYLE["accent_max_station_gap"])+1)
        stations.update(a[0]+(b[0]-a[0])*i/n for i in range(1,n))
    return [(t,curve(t,1,rails),curve(t,2,rails)) for t in sorted(stations)]


def signed_volume(vertices, faces):
    origin = vertices[0]
    volume = 0.0
    for face in faces:
        for j in range(1, len(face) - 1):
            a, b, c = [tuple(vertices[i][k] - origin[k] for k in range(3))
                       for i in (face[0], face[j], face[j + 1])]
            volume += (a[0]*(b[1]*c[2]-b[2]*c[1])
                       - a[1]*(b[0]*c[2]-b[2]*c[0])
                       + a[2]*(b[0]*c[1]-b[1]*c[0])) / 6
    return volume


def closed_ribbon_mesh(sections, point, thickness, roll, ridge_fraction=.42, folded_return=False):
    """Matched six-rail thin return; pointed ends use bounded-valence caps, not tiny loops."""
    vertices, rings, faces, zones = [], [], [], []
    for t, lower, upper in sections:
        if upper - lower < 1e-9:
            rings.append([len(vertices)])
            vertices.append(point(t, lower, 0.0))
            continue
        ridge = lower + (upper - lower) * ridge_fraction
        relief = (roll(t) if callable(roll) else roll) * min(1.0, (upper - lower) / .014)
        section = [(lower, -thickness), (lower, 0.0), (ridge, relief),
                   (upper, 0.0), (upper, -thickness), (ridge, relief-thickness if folded_return else -thickness)]
        rings.append(list(range(len(vertices), len(vertices) + len(section))))
        vertices.extend(point(t, height, offset) for height, offset in section)
    for first, second in zip(rings, rings[1:]):
        assert len(first) + len(second) > 2, "Adjacent collapsed stations"
        for lane in range(6):
            nxt = (lane + 1) % 6
            if len(first) == 1:
                face = (first[0], second[nxt], second[lane])
            elif len(second) == 1:
                face = (first[lane], first[nxt], second[0])
            else:
                face = (first[lane], first[nxt], second[nxt], second[lane])
            if len(face) == 4:
                # Match front/return diagonals in physical space. Opposite
                # winding must not choose a different split on a warped quad.
                if folded_return and lane in (4, 5):
                    face = face[1:] + face[:1]
                faces.extend([(face[0], face[1], face[2]), (face[0], face[2], face[3])])
                zones.extend([lane, lane])
            else:
                faces.append(face)
                zones.append(lane)
    for ring, reverse in [(rings[0], True), (rings[-1], False)]:
        if len(ring) > 1:
            faces.append(tuple(reversed(ring)) if reverse else tuple(ring))
            zones.append(6)
    volume = signed_volume(vertices, faces)
    assert abs(volume) > 1e-14, "Collapsed accent volume"
    if volume < 0:
        faces = [(face[0], *reversed(face[1:])) for face in faces]
    return vertices, faces, zones


def accent_mesh(name, sections, point, color, thickness, roll, folded=False):
    vertices, faces, zones = closed_ribbon_mesh(sections, point, thickness, roll,
        STYLE["accent_fold_ridge_fraction"] if folded else .42, folded)
    obj = mesh(name, vertices, faces, color)
    attribute = obj.data.attributes.new("AccentNormalZone", "INT", "FACE")
    front = []
    for polygon, zone in zip(obj.data.polygons, zones):
        attribute.data[polygon.index].value = zone
        polygon.use_smooth = zone in (1, 2)
        if zone in (1, 2):
            front.append(polygon.index)
    obj["accent_front_faces"] = front
    obj["accent_positive_volume_m3"] = abs(signed_volume(vertices, faces))
    assert all(p.area > 1e-12 for p in obj.data.polygons), "Degenerate accent face"
    return obj




def tuft_rear_scoop(progress, maximum):
    """A rooted leaf retreats behind its parent before its free tip curls forward."""
    p=max(0.0,min(1.0,progress))
    return maximum*sin(pi*p/.45)**2 if 0.0 < p < .45 else 0.0


def tuft_free_bow(progress, maximum):
    p=max(0.0,min(1.0,(progress-.25)/.75))
    return maximum*sin(pi*p)**2


