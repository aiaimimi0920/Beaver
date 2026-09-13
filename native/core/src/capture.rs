use anyhow::{ensure, Result};

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::Capture;

#[cfg(not(windows))]
#[derive(Default)]
pub struct Capture;
#[cfg(not(windows))]
impl Capture {
    pub fn sources(
        &mut self,
        _cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<serde_json::Value> {
        anyhow::bail!("此平台的原生窗口截图尚未实现")
    }
    pub fn capture(
        &self,
        _id: &str,
        _cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<Vec<u8>> {
        anyhow::bail!("此平台的原生窗口截图尚未实现")
    }
}

fn dimensions(width: u32, height: u32) -> Result<()> {
    ensure!(width > 0 && height > 0, "窗口没有可采集的画面");
    ensure!(
        u64::from(width) * u64::from(height) <= 32 * 1024 * 1024,
        "窗口画面超过截图尺寸上限"
    );
    Ok(())
}

fn encode_bgra(
    width: u32,
    height: u32,
    stride: usize,
    bytes: &[u8],
    bounds: (u32, u32),
) -> Result<Vec<u8>> {
    dimensions(width, height)?;
    ensure!(bounds.0 > 0 && bounds.1 > 0, "截图输出尺寸无效");
    let row = width as usize * 4;
    ensure!(stride >= row, "截图像素行长度无效");
    let required = stride
        .checked_mul(height as usize - 1)
        .and_then(|n| n.checked_add(row))
        .ok_or_else(|| anyhow::anyhow!("截图像素长度溢出"))?;
    ensure!(bytes.len() >= required, "截图像素数据不完整");
    let mut rgba = image::RgbaImage::new(width, height);
    for y in 0..height as usize {
        for (source, target) in bytes[y * stride..y * stride + row]
            .chunks_exact(4)
            .zip(rgba.as_mut()[y * row..(y + 1) * row].chunks_exact_mut(4))
        {
            target.copy_from_slice(&[source[2], source[1], source[0], source[3]]);
        }
    }
    let image =
        image::DynamicImage::ImageRgba8(rgba).thumbnail(bounds.0.min(width), bounds.1.min(height));
    let mut png = std::io::Cursor::new(Vec::new());
    image.write_to(&mut png, image::ImageFormat::Png)?;
    Ok(png.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn png_conversion_preserves_channels_and_ignores_row_padding() -> Result<()> {
        let png = encode_bgra(
            1,
            2,
            8,
            &[0, 0, 255, 255, 9, 9, 9, 9, 255, 0, 0, 255],
            (640, 360),
        )?;
        let image = image::load_from_memory(&png)?.into_rgba8();
        assert_eq!(image.dimensions(), (1, 2));
        assert_eq!(image.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(image.get_pixel(0, 1).0, [0, 0, 255, 255]);
        assert!(encode_bgra(1, 2, 8, &[0; 11], (1, 2)).is_err());
        assert!(encode_bgra(1, 1, 3, &[0; 4], (1, 1)).is_err());
        assert!(encode_bgra(0, 1, 0, &[], (1, 1)).is_err());
        assert!(encode_bgra(u32::MAX, u32::MAX, 0, &[], (1, 1)).is_err());
        let png = encode_bgra(4, 2, 16, &[255; 32], (2, 2))?;
        assert_eq!(
            image::load_from_memory(&png)?.into_rgba8().dimensions(),
            (2, 1)
        );
        Ok(())
    }
}
