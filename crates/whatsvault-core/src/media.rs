use std::path::Path;

use crate::AttachmentKind;

pub fn attachment_kind_from_filename(filename: &str) -> AttachmentKind {
    match extension(filename).as_str() {
        "gif" => AttachmentKind::Gif,
        "jpg" | "jpeg" | "png" | "heic" | "webp" => AttachmentKind::Photo,
        "mp3" | "m4a" | "ogg" | "opus" | "wav" => AttachmentKind::Audio,
        "mov" | "mp4" | "webm" => AttachmentKind::Video,
        _ => AttachmentKind::Unknown,
    }
}

pub fn attachment_kind_from_mime_or_filename(
    media_type: Option<&str>,
    filename: &str,
) -> AttachmentKind {
    let normalized_media_type = media_type
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase);

    match normalized_media_type.as_deref() {
        Some("image/gif") => AttachmentKind::Gif,
        Some(value) if value.starts_with("image/") => AttachmentKind::Photo,
        Some(value) if value.starts_with("audio/") => AttachmentKind::Audio,
        Some(value) if value.starts_with("video/") => AttachmentKind::Video,
        _ => attachment_kind_from_filename(filename),
    }
}

pub fn attachment_media_type(kind: AttachmentKind, filename: &str) -> Option<&'static str> {
    match kind {
        AttachmentKind::Audio => audio_media_type(filename),
        AttachmentKind::Gif => Some("image/gif"),
        AttachmentKind::Photo | AttachmentKind::Sticker => image_media_type(filename),
        AttachmentKind::Video => video_media_type(filename),
        AttachmentKind::Unknown => fallback_media_type(filename),
    }
}

fn image_media_type(filename: &str) -> Option<&'static str> {
    match extension(filename).as_str() {
        "gif" => Some("image/gif"),
        "heic" => Some("image/heic"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

fn audio_media_type(filename: &str) -> Option<&'static str> {
    match extension(filename).as_str() {
        "m4a" => Some("audio/mp4"),
        "mp3" => Some("audio/mpeg"),
        "ogg" | "opus" => Some("audio/ogg"),
        "wav" => Some("audio/wav"),
        _ => None,
    }
}

fn video_media_type(filename: &str) -> Option<&'static str> {
    match extension(filename).as_str() {
        "mov" => Some("video/quicktime"),
        "mp4" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        _ => None,
    }
}

fn fallback_media_type(filename: &str) -> Option<&'static str> {
    match extension(filename).as_str() {
        "pdf" => Some("application/pdf"),
        _ => None,
    }
}

fn extension(filename: &str) -> String {
    Path::new(filename)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}
