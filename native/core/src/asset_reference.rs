use crate::{
    asset_task::{self, Annotation, Frame, Reference},
    files::safe_path,
    store::Store,
};
use anyhow::{bail, Context, Result};
use image::{ImageEncoder, Rgba, RgbaImage};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};

pub const MAX_IMAGE: usize = 4 * 1024 * 1024;

pub fn capture(
    store: &Store,
    root: &Path,
    task_id: &str,
    frame: Frame,
    bytes: &[u8],
) -> Result<Reference> {
    let mut state = asset_task::get(store, task_id)?;
    if state.session_id.as_deref() != Some(&frame.session_id) {
        bail!("Preview session changed");
    }
    if bytes.len() > MAX_IMAGE
        || frame.width == 0
        || frame.height == 0
        || frame.width > 1280
        || frame.height > 960
        || frame
            .view_matrix
            .iter()
            .chain(&frame.projection_matrix)
            .any(|v| !v.is_finite())
    {
        bail!("Invalid preview frame");
    }
    let decoded = decode(bytes)?;
    if decoded.width() != frame.width || decoded.height() != frame.height {
        bail!("Frame dimensions do not match pixels");
    }
    let references: Vec<Reference> = store.list("asset-reference")?;
    let mut unused: Vec<_> = references
        .iter()
        .filter(|r| r.task_id == task_id && !r.used)
        .collect();
    unused.sort_by_key(|r| r.frame.captured_at);
    for old in unused.iter().take(unused.len().saturating_sub(7)) {
        let path = safe_path(
            root,
            &format!("asset-observer/{task_id}/references/{}.png", old.id),
        )?;
        if path.exists() {
            fs::remove_file(path)?;
        }
        store.remove("asset-reference", &old.id)?;
    }
    if references
        .iter()
        .filter(|r| r.task_id == task_id && r.used)
        .count()
        >= 2 * asset_task::MAX_FEEDBACK + 4
    {
        bail!("Task reference retention limit reached");
    }
    let id = uuid::Uuid::new_v4().to_string();
    let path = safe_path(
        root,
        &format!("asset-observer/{task_id}/references/{id}.png"),
    )?;
    fs::create_dir_all(path.parent().context("Reference directory missing")?)?;
    fs::write(&path, bytes)?;
    let reference = Reference {
        id,
        task_id: task_id.into(),
        project_id: state.project_id.clone(),
        frame,
        pick: None,
        image_path: path.to_string_lossy().into_owned(),
        sha256: format!("{:x}", Sha256::digest(bytes)),
        used: false,
    };
    store.put("asset-reference", &reference.id, &reference)?;
    state.last_frame = Some(reference.clone());
    asset_task::save(store, &state)?;
    Ok(reference)
}

pub fn get(store: &Store, task_id: &str, id: &str) -> Result<Reference> {
    asset_task::validate_id(id)?;
    let reference: Reference = store
        .get("asset-reference", id)?
        .context("Reference image expired; freeze a new frame")?;
    if reference.task_id != task_id {
        bail!("Reference belongs to another task");
    }
    Ok(reference)
}

pub fn read(reference: &Reference) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(&reference.image_path)?
        .take((MAX_IMAGE + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_IMAGE || format!("{:x}", Sha256::digest(&bytes)) != reference.sha256 {
        bail!("Reference image integrity check failed");
    }
    Ok(bytes)
}

pub fn same_scene(reference: &Frame, current: &Frame) -> Result<()> {
    if reference.session_id != current.session_id
        || reference.generation != current.generation
        || reference.scene_revision != current.scene_revision
    {
        bail!("Reference frame expired; refresh and select again");
    }
    Ok(())
}

pub fn set_pick(
    store: &Store,
    task_id: &str,
    id: &str,
    pick: crate::asset_task::Pick,
) -> Result<Reference> {
    let mut reference = get(store, task_id, id)?;
    if reference.used {
        bail!("Submitted reference is immutable; freeze another frame");
    }
    if pick.frame_id != reference.frame.id {
        bail!("Pick belongs to another frame");
    }
    reference.pick = Some(pick);
    store.put("asset-reference", id, &reference)?;
    Ok(reference)
}

pub fn validate_annotations(annotations: &[Annotation]) -> Result<()> {
    if annotations.len() > 64 || annotations.iter().map(|a| a.points.len()).sum::<usize>() > 4096 {
        bail!("Too many annotation points");
    }
    for mark in annotations {
        if !["box", "arrow", "brush"].contains(&mark.kind.as_str())
            || mark.points.len() < 2
            || (mark.kind != "brush" && mark.points.len() != 2)
            || mark
                .points
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            bail!("Annotations require normalized image coordinates");
        }
    }
    Ok(())
}

pub fn annotated(reference: &Reference, annotations: &[Annotation]) -> Result<Vec<u8>> {
    validate_annotations(annotations)?;
    let bytes = read(reference)?;
    if annotations.is_empty() && reference.pick.is_none() {
        return Ok(bytes);
    }
    let mut image = decode(&bytes)?.to_rgba8();
    let point = |p: [f64; 2]| {
        [
            (p[0] * (reference.frame.width - 1) as f64) as i32,
            (p[1] * (reference.frame.height - 1) as f64) as i32,
        ]
    };
    for mark in annotations {
        let a = point(mark.points[0]);
        let b = point(mark.points[1]);
        if mark.kind == "box" {
            for (s, e) in [
                (a, [b[0], a[1]]),
                ([b[0], a[1]], b),
                (b, [a[0], b[1]]),
                ([a[0], b[1]], a),
            ] {
                line(&mut image, s, e);
            }
        } else {
            for pair in mark.points.windows(2) {
                line(&mut image, point(pair[0]), point(pair[1]));
            }
            if mark.kind == "arrow" {
                let angle = ((a[1] - b[1]) as f64).atan2((a[0] - b[0]) as f64);
                for d in [-0.45, 0.45] {
                    line(
                        &mut image,
                        b,
                        [
                            b[0] + ((angle + d).cos() * 16.0) as i32,
                            b[1] + ((angle + d).sin() * 16.0) as i32,
                        ],
                    );
                }
            }
        }
    }
    if let Some(pick) = &reference.pick {
        let p = point(pick.point);
        line(&mut image, [p[0] - 9, p[1]], [p[0] + 9, p[1]]);
        line(&mut image, [p[0], p[1] - 9], [p[0], p[1] + 9]);
    }
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png).write_image(
        image.as_raw(),
        image.width(),
        image.height(),
        image::ExtendedColorType::Rgba8,
    )?;
    Ok(png)
}

fn decode(bytes: &[u8]) -> Result<image::DynamicImage> {
    let mut reader =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(1280);
    limits.max_image_height = Some(960);
    limits.max_alloc = Some(16 * 1024 * 1024);
    reader.limits(limits);
    Ok(reader.decode()?)
}

fn line(image: &mut RgbaImage, a: [i32; 2], b: [i32; 2]) {
    let steps = (b[0] - a[0]).abs().max((b[1] - a[1]).abs()).max(1);
    for step in 0..=steps {
        let x = a[0] + (b[0] - a[0]) * step / steps;
        let y = a[1] + (b[1] - a[1]) * step / steps;
        for dx in -1..=1 {
            for dy in -1..=1 {
                if x + dx >= 0
                    && y + dy >= 0
                    && x + dx < image.width() as i32
                    && y + dy < image.height() as i32
                {
                    image.put_pixel((x + dx) as u32, (y + dy) as u32, Rgba([255, 195, 50, 255]));
                }
            }
        }
    }
}
