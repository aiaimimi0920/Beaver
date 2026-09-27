use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Selection {
    kind: String,
    sequence: u64,
    sha256: String,
    regions: Vec<Region>,
    prompt: String,
    coordinate_space: String,
    hit_capability: String,
    #[serde(default)]
    picks: Vec<Pick>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pick {
    region: usize,
    result: Value,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Region {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    prompt: String,
}

pub(super) fn validate(selection: &Value, frame: &Value) -> Result<()> {
    let value: Selection =
        serde_json::from_value(selection.clone()).context("PREVIEW_SELECTION_INVALID")?;
    ensure!(
        value.kind == "image-regions"
            && value.coordinate_space == "normalized-image"
            && matches!(
                value.hit_capability.as_str(),
                "unavailable" | "frozen-static-mesh-ray" | "frozen-static-mesh"
            ),
        "PREVIEW_SELECTION_CAPABILITY_INVALID"
    );
    ensure!(
        frame["frozen"] == true
            && frame["sequence"] == value.sequence
            && frame["sha256"] == value.sha256,
        "PREVIEW_SELECTION_FRAME_MISMATCH"
    );
    ensure!(
        !value.regions.is_empty() && value.regions.len() <= 8 && value.prompt.len() <= 4000,
        "PREVIEW_SELECTION_LIMIT"
    );
    ensure!(
        (value.picks.is_empty() && value.hit_capability == "unavailable")
            || (!value.picks.is_empty()
                && matches!(
                    value.hit_capability.as_str(),
                    "frozen-static-mesh-ray" | "frozen-static-mesh"
                )),
        "PREVIEW_SELECTION_CAPABILITY_INVALID"
    );
    let mut indices = std::collections::BTreeSet::new();
    for pick in &value.picks {
        ensure!(
            pick.region < value.regions.len() && indices.insert(pick.region),
            "PREVIEW_SELECTION_PICK_INVALID"
        );
        super::live_preview_pick::validate(&pick.result, frame)?;
        ensure!(
            frame["picks"]
                .as_array()
                .is_some_and(|picks| picks.contains(&pick.result)),
            "PREVIEW_SELECTION_PICK_UNTRUSTED"
        );
        let region = &value.regions[pick.region];
        if let Some(rectangle) = pick.result.get("rectangle") {
            ensure!(
                value.hit_capability == "frozen-static-mesh"
                    && rectangle["x"].as_f64() == Some(region.x)
                    && rectangle["y"].as_f64() == Some(region.y)
                    && rectangle["width"].as_f64() == Some(region.width)
                    && rectangle["height"].as_f64() == Some(region.height),
                "PREVIEW_SELECTION_PICK_OUTSIDE"
            );
        }
        let x = pick.result["point"]["x"].as_f64().unwrap();
        let y = pick.result["point"]["y"].as_f64().unwrap();
        ensure!(
            x >= region.x
                && x <= region.x + region.width
                && y >= region.y
                && y <= region.y + region.height,
            "PREVIEW_SELECTION_PICK_OUTSIDE"
        );
    }
    for region in value.regions {
        ensure!(
            [region.x, region.y, region.width, region.height]
                .iter()
                .all(|v| v.is_finite())
                && region.x >= 0.0
                && region.y >= 0.0
                && region.width > 0.0
                && region.height > 0.0
                && region.x + region.width <= 1.0
                && region.y + region.height <= 1.0
                && region.prompt.len() <= 1000,
            "PREVIEW_SELECTION_REGION_INVALID"
        );
    }
    Ok(())
}
