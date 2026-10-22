"""Fixed sampled native review schedule, not a speech dataset."""

import json
from pathlib import Path


def schedule():
    poses = [
        {},
        {"MouthOpen": 1.0},
        {"MouthOpen": 0.7, "MouthRound": 1.0},
        {"MouthOpen": 0.5, "MouthSpread": 1.0},
        {"MouthPucker": 1.0},
        {"MouthOpen": 0.5, "MouthPucker": 1.0},
        {"MouthOpen": 0.6, "MouthRound": 0.5, "MouthSpread": 0.5},
        {"MouthOpen": 1.0, "MouthRound": 1.0, "MouthSpread": 1.0, "MouthPucker": 1.0},
    ]
    result = []
    for pose in poses:
        for yaw in [0.0, -45.0, 45.0, -90.0, 90.0]:
            for mode in ["render", "white"]:
                result.append(
                    {
                        "mouth": pose,
                        "yaw": yaw,
                        "pitch": 0.0,
                        "mode": mode,
                        "blink": [0.0, 0.0],
                    }
                )
    for pose, blink in [(poses[2], [1.0, 1.0]), (poses[5], [1.0, 0.0])]:
        for yaw in [0.0, -45.0, 45.0]:
            for mode in ["render", "white"]:
                result.append(
                    {
                        "mouth": pose,
                        "yaw": yaw,
                        "pitch": 0.0,
                        "mode": mode,
                        "blink": blink,
                    }
                )
    return result


if __name__ == "__main__":
    (Path(__file__).parent / "runtime/mouth_capture_schedule.json").write_text(
        json.dumps(schedule(), indent=2) + "\n"
    )
