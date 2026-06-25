use whatsvault_core::{
    media::{
        attachment_display_label, attachment_kind_from_filename,
        attachment_kind_from_mime_or_filename,
    },
    AttachmentKind,
};

#[test]
fn classifies_attachment_kind_from_filename_extensions() {
    assert_eq!(
        attachment_kind_from_filename("photo.JPG"),
        AttachmentKind::Photo
    );
    assert_eq!(
        attachment_kind_from_filename("voice.opus"),
        AttachmentKind::Audio
    );
    assert_eq!(
        attachment_kind_from_filename("clip.mp4"),
        AttachmentKind::Video
    );
    assert_eq!(
        attachment_kind_from_filename("animated.gif"),
        AttachmentKind::Gif
    );
    assert_eq!(
        attachment_kind_from_filename("document.pdf"),
        AttachmentKind::Unknown
    );
}

#[test]
fn prefers_mime_type_when_it_is_more_specific_than_filename() {
    assert_eq!(
        attachment_kind_from_mime_or_filename(Some("audio/ogg"), "media.bin"),
        AttachmentKind::Audio
    );
    assert_eq!(
        attachment_kind_from_mime_or_filename(Some("image/jpeg"), "media.bin"),
        AttachmentKind::Photo
    );
    assert_eq!(
        attachment_kind_from_mime_or_filename(Some("video/mp4"), "media.bin"),
        AttachmentKind::Video
    );
    assert_eq!(
        attachment_kind_from_mime_or_filename(None, "sticker.webp"),
        AttachmentKind::Photo
    );
}

#[test]
fn exposes_human_attachment_labels_for_chat_previews() {
    assert_eq!(attachment_display_label(AttachmentKind::Photo), "Photo");
    assert_eq!(
        attachment_display_label(AttachmentKind::Audio),
        "Voice message"
    );
    assert_eq!(
        attachment_display_label(AttachmentKind::Unknown),
        "Attachment"
    );
}
