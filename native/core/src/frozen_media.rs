//! Passive media formats permitted in frozen file previews.
use base64::{engine::general_purpose::STANDARD, Engine};
use std::path::Path;

pub enum Media {
    Image { mime: String, base64: String },
    Audio { mime: String, base64: String },
}

pub fn classify(path: &str, bytes: &[u8]) -> Option<Media> {
    let extension = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
    let (image, mime) = match extension.as_str() {
        "png" => (true, "image/png"),
        "jpg" | "jpeg" => (true, "image/jpeg"),
        "gif" => (true, "image/gif"),
        "webp" => (true, "image/webp"),
        "wav" => (false, "audio/wav"),
        "mp3" => (false, "audio/mpeg"),
        "ogg" => (false, "audio/ogg"),
        "flac" => (false, "audio/flac"),
        _ => return None,
    };
    let mime = mime.into();
    let base64 = STANDARD.encode(bytes);
    Some(if image {
        Media::Image { mime, base64 }
    } else {
        Media::Audio { mime, base64 }
    })
}
