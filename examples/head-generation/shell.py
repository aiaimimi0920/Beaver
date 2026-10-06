from face import *


def upper_boundary_offset(y, q):
    v = (y - Y0) / H
    blend = max(0, min(1, (v - C["top_warp_start_height"]) / (1 - C["top_warp_start_height"])))
    blend = blend * blend * (3 - 2 * blend)
    return H * curve(q, 1, C["top_boundary_drop_profile"]) * blend


def upper_boundary_depth(x, warped_y, original_y, q):
    v = (original_y - Y0) / H
    blend = max(0, min(1, (v - C["top_warp_start_height"]) / (1 - C["top_warp_start_height"])))
    blend = blend * blend * (3 - 2 * blend)
    target = Z0 + H * curve(q, 1, C["top_boundary_depth_profile"])
    top_y = Y0 + H - H * curve(q, 1, C["top_boundary_drop_profile"])
    top_x = width(top_y) * q
    correction = target - depth(top_x, top_y)
    return depth(x, warped_y) + correction * blend


def jaw_return_position(x, y, z, q, boundary_depth):
    start = C["jaw_return_start_ratio"]
    blend = max(0, min(1, (q - start) / (1 - start)))
    blend = blend * blend * (3 - 2 * blend)
    v = (y - Y0) / H
    low = max(
        0,
        min(
            1,
            (v - C["jaw_return_low_fade_start"])
            / (C["jaw_return_low_fade_end"] - C["jaw_return_low_fade_start"]),
        ),
    )
    blend *= low * low * (3 - 2 * low)
    requested_inset = H * curve(v, 1, C["jaw_return_width_profile"])
    max_inset = abs(x) / max(q, 0.00001) * C["jaw_return_max_inset_ratio"]
    inset = min(requested_inset, max_inset) * blend
    fade = max(
        0,
        min(
            1,
            (C["jaw_return_depth_fade_end"] - v)
            / (C["jaw_return_depth_fade_end"] - C["jaw_return_depth_fade_start"]),
        ),
    )
    fade = fade * fade * (3 - 2 * fade)
    target = Z0 + H * curve(v, 1, C["jaw_return_depth_profile"])
    return x - inset, z + (target - boundary_depth) * blend * fade * C["jaw_return_depth_strength"]


def chin_underside_height(lift, q):
    return Y0 + H * (lift + C["chin_return_lift"] * q * q)


def chin_underside_depth_offset(back, lift, q):
    blend = max(0, min(1, lift / 0.03))
    blend = blend * blend * (3 - 2 * blend)
    radius = C["chin_underside_roundness"]
    profile = (sqrt(q * q + radius * radius) - radius) / (sqrt(1 + radius * radius) - radius)
    rear_fade = max(0, min(1, (C["chin_underside_stations"][-1][1] - lift) / 0.02))
    return H * (back + C["chin_underside_center_back"] * blend * rear_fade * (1 - profile))


def build_face():
    levels = [
        1.544,
        1.551,
        1.560,
        1.570,
        1.580,
        1.590,
        1.599,
        1.608,
        1.614,
        1.623,
        1.633,
        1.644,
        1.655,
        1.664,
        1.673,
        1.685,
        1.701,
        1.720,
        1.740,
        1.760,
        1.780,
    ]
    assert min(b - a for a, b in zip(levels, levels[1:])) >= 0.0059
    lateral = [0, 0.06, 0.12, 0.18, 0.25, 0.34, 0.44, 0.55, 0.67, 0.78, 0.87, 0.94, 1]
    rows, cols = len(levels), len(lateral)
    verts = []
    for j in range(rows):
        y = levels[j]
        for i in range(cols):
            focus = max(0, min(1, (y - 1.544) / 0.026)) * max(0, min(1, (1.710 - y) / 0.028))
            q = lateral[i] * focus + (i / (cols - 1)) * (1 - focus)
            x = width(y) * q
            top_blend = float(y > Y0 + H * C["top_warp_start_height"])
            yy = (
                y
                - upper_boundary_offset(y, q)
                + H * C["chin_return_lift"] * q * q * exp(-(y - 1.544) / 0.008)
                + H * C["chin_front_lift"] * exp(-(y - Y0) / (0.02 * H))
            )
            # Re-evaluate width after height warping; otherwise the outer rim
            # incorrectly samples an interior depth and develops a notch.
            x = width(yy) * q if top_blend > 0 else x
            zz = upper_boundary_depth(x, yy, y, q)
            x, zz = jaw_return_position(x, yy, zz, q, depth(width(yy), yy))
            verts.append(coord(x, yy, zz))
    # Feature-aligned patches replace the uniform lattice around eyes and mouth.
    ej0 = levels.index(1.614)
    ej1 = levels.index(1.673)
    ei0, ei1 = 3, 11
    mj0 = levels.index(1.570)
    mj1 = levels.index(1.599)
    mi1 = 6
    shell_faces = []
    for j in range(rows - 1):
        for i in range(cols - 1):
            if ej0 <= j < ej1 and ei0 <= i < ei1:
                continue
            if mj0 <= j < mj1 and i < mi1:
                continue
            shell_faces.append(
                (j * cols + i, j * cols + i + 1, (j + 1) * cols + i + 1, (j + 1) * cols + i)
            )

    def patch(boundary, kind, closed):
        # Shared outer vertices keep the feature rings connected to the facial shell.
        inner = []
        for vi in boundary:
            x, _, y = verts[vi]
            if kind == "eye":
                a = math.atan2(
                    (y - (Y0 + H * C["eye_corner_y"])) / 0.030,
                    (x - STYLE["eye_center_x_m"]) / 0.040,
                )
                t = cos(a)
                xx, yy = eye_contour(1, t, sin(a) >= 0)
            else:
                a = math.atan2((y - 1.581) / 0.018, x / 0.035)
                xx = 0.0195 * cos(a)
                yy = 1.581 + 0.0011 * sin(a)
            inner.append((xx, yy))
        if kind == "mouth":
            from mouth import mouth_half_contour

            inner = [
                mouth_half_contour(-pi / 2 + pi * k / (len(boundary) - 1))
                for k in range(len(boundary))
            ]
        rings = []
        for blend in ([0, 0.30, 0.65] if kind == "eye" else [0, 0.30, 0.65]):
            ring = []
            for k, vi in enumerate(boundary):
                bx, _, by = verts[vi]
                ix, iy = inner[k]
                x = ix * (1 - blend) + bx * blend
                y = iy * (1 - blend) + by * blend
                dz = 0
                if kind == "mouth":
                    a = -pi / 2 + pi * k / (len(boundary) - 1)
                    rim_key = (
                        "upper_lip_edge_depth_delta"
                        if sin(a) >= 0
                        else "lower_lip_edge_depth_delta"
                    )
                    dz = H * C[rim_key] * abs(sin(a)) * (1 - blend) ** 2
                ring.append(len(verts))
                verts.append(coord(x, y, depth(x, y) + dz))
            rings.append(ring)
        rings.append(boundary)
        for ra, rb in zip(rings, rings[1:]):
            for k in range(len(boundary) if closed else len(boundary) - 1):
                kn = (k + 1) % len(boundary)
                shell_faces.append((ra[k], rb[k], rb[kn], ra[kn]))
        if kind == "mouth":
            from mouth import build_oral_cavity

            build_oral_cavity([verts[i] for i in rings[0]])

    eye_boundary = (
        [ej0 * cols + i for i in range(ei0, ei1 + 1)]
        + [j * cols + ei1 for j in range(ej0 + 1, ej1 + 1)]
        + [ej1 * cols + i for i in range(ei1 - 1, ei0 - 1, -1)]
        + [j * cols + ei0 for j in range(ej1 - 1, ej0, -1)]
    )
    patch(eye_boundary, "eye", True)
    mouth_boundary = (
        [mj0 * cols + i for i in range(mi1 + 1)]
        + [j * cols + mi1 for j in range(mj0 + 1, mj1 + 1)]
        + [mj1 * cols + i for i in range(mi1 - 1, -1, -1)]
    )
    patch(mouth_boundary, "mouth", False)
    front_count = len(shell_faces)
    base = [verts[i] for i in range(cols)]
    previous = list(range(cols))
    side_path = [cols - 1]
    for back, lift, spread in C["chin_underside_stations"]:
        current = []
        for i, (x, negz, y) in enumerate(base):
            q = i / (cols - 1)
            current.append(len(verts))
            half_width = min(
                base[-1][0] * spread, width(Y0 + H * lift) * C["chin_return_boundary_inset"]
            )
            verts.append(
                (
                    half_width * q,
                    negz + chin_underside_depth_offset(back, lift, q),
                    chin_underside_height(lift, q),
                )
            )
        for i in range(cols - 1):
            shell_faces.append((previous[i + 1], previous[i], current[i], current[i + 1]))
        previous = current
        side_path.append(current[-1])
    underside_count = len(shell_faces) - front_count
    outer = [j * cols + cols - 1 for j in range(len(side_path))]
    shell_faces.append((outer[0], side_path[1], outer[1]))
    for j in range(1, len(side_path) - 1):
        shell_faces.append((side_path[j], side_path[j + 1], outer[j + 1], outer[j]))
    face = mesh("Face shell editable half", verts, shell_faces, SKIN, mirror=True)
    face["front_surface_face_count"] = front_count
    face["underside_face_count"] = underside_count

    group = face.vertex_groups.new(name="Continuous facial shell")
    group.add(list(range(len(verts))), 1.0, "REPLACE")
    from ears import build_ears

    build_ears()
    return {
        "face_authoring_quads": sum(len(q) == 4 for q in shell_faces),
        "face_authoring_triangles": sum(len(q) == 3 for q in shell_faces),
        "semantic_rows": levels,
        "chin_underside_quads": underside_count,
        "chin_side_quads": len(side_path) - 2,
        "chin_side_triangles": 1,
    }
