"""Apply opt-in shared upper shape to derived sources, never the checkpoint."""

from pathlib import Path

HERE = Path(__file__).resolve().parent
HELPERS = (HERE / "contour_warp.py").read_text()
HELPER_NAMES = {
    "eye_warp",
    "eye_warp_derivative",
    "upper_shape",
    "upper_shape_derivative",
}


def once(source, old, new):
    if source.count(old) != 1:
        raise ValueError("Upper shape source contract changed: " + old[:80])
    return source.replace(old, new, 1)


def adapt_face_source(source, enabled):
    if not enabled:
        return source
    source = once(
        source,
        "    return (x, Y0 + H * (cy + h * max(0, 1 - abs(t) ** horizontal) ** power))",
        "    shape = upper_shape(t, C) if upper else max(0, 1 - abs(t) ** horizontal) ** power\n"
        "    return (x, Y0 + H * (cy + h * shape))",
    )
    source = once(
        source,
        "    r = sqrt(abs(t) ** horizontal + abs((y - cy) / (H * arch)) ** (1 / power))",
        "    q = eye_warp(t, C) if y >= cy else t\n"
        "    r = sqrt(abs(q) ** horizontal + abs((y - cy) / (H * arch)) ** (1 / power))",
    )
    return HELPERS + "\n\n" + source


def adapt_upper_sources(sources, enabled):
    if not enabled:
        return sources
    sources = dict(sources)
    sources["face.py"] = adapt_face_source(sources["face.py"], True)
    socket = once(
        sources["eye_socket.py"],
        "                    abs(t)\n",
        "                    abs(eye_warp(t, C) if y >= cy else t)\n",
    )
    # Keep the inherited pocket vertical classifier, including its explicit
    # lower-roundness compatibility mode. Only the upper horizontal term moves.
    sources["eye_socket.py"] = "from face import eye_warp\n" + socket
    lateral = once(
        sources["lateral_fit.py"],
        '        return y0+h*(c["eye_corner_y"]+c["eye_corner_slope"]*t+arch*max(0,1-t**horizontal)**power)',
        "        shape=upper_shape(t,c) if upper else max(0,1-t**horizontal)**power\n"
        '        return y0+h*(c["eye_corner_y"]+c["eye_corner_slope"]*t+arch*shape)',
    )
    sources["lateral_fit.py"] = "from face import upper_shape\n" + lateral
    return sources
