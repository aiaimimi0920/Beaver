"""Opt-in skin-tethered side-lash guide; keep legacy construction at weight one."""

import math


def adapt_regularization(source, weight):
    if isinstance(weight, bool) or not isinstance(weight, (int, float)):
        raise ValueError("Guide regularization must be finite numeric")
    if not math.isfinite(weight) or not 0 <= weight <= 1:
        raise ValueError("Guide regularization must be in [0,1]")
    if weight == 1:
        return source

    def once(text, old, new):
        if text.count(old) != 1:
            raise ValueError(
                "Lateral regularization source contract changed: " + old[:60]
            )
        return text.replace(old, new, 1)

    begin = source.index("def continuous_guide_moves(")
    end = source.index("\ndef wing_depth_shift", begin)
    function = source[begin:end]
    function = once(
        function,
        "inward_power=2.0):",
        f"inward_power=2.0,regularization={weight!r},return_raw=False):",
    )
    function = once(
        function,
        "    moves=[]",
        "    moves=[]\n    raw_max=0.0\n    assert not isinstance(regularization,bool) and math.isfinite(regularization) and 0<=regularization<=1",
    )
    function = once(
        function,
        "        delta=tuple(target[i]-base[i] for i in range(3))",
        "        raw_delta=tuple(target[i]-base[i] for i in range(3))\n"
        "        raw_max=max(raw_max,math.dist((0,0,0),raw_delta))\n"
        "        delta=tuple(d*regularization for d in raw_delta)",
    )
    function = once(
        function,
        "    return moves",
        "    return (moves,raw_max) if return_raw else moves",
    )
    source = source[:begin] + function + source[end:]
    call = '        guide_moves=continuous_guide_moves(stations,seated_bases,style["accent_lateral_guide_bow_fraction"],style["accent_lateral_guide_inward_power"])'
    source = once(
        source,
        call,
        call.replace("guide_moves=", "guide_moves,guide_raw_max=")[:-1]
        + ",return_raw=True)",
    )
    source = once(
        source,
        "        for ring,delta in zip(rings,guide_moves):",
        "        validate_combined_guide_moves(guide_moves)\n"
        "        for ring,delta in zip(rings,guide_moves):",
    )
    source = once(
        source,
        'report[str(side)]={"maximum_companion_parent_pairing_m":',
        f'report[str(side)]={{"guide_regularization":{weight!r},"maximum_unweighted_endpoint_guide_m":guide_raw_max,"maximum_companion_parent_pairing_m":',
    )
    source += """

def validate_combined_guide_moves(moves):
    assert moves and all(len(d)==3 and all(math.isfinite(v) for v in d) for d in moves)
    maximum=max(math.dist((0,0,0),d) for d in moves)
    assert maximum < .004, ("Combined lateral guide requires structural redesign", maximum)
    return maximum
"""
    compile(source, "<skin-tethered lateral guide>", "exec")
    return source
