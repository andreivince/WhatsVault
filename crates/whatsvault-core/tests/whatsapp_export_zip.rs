use std::io::{Cursor, Write};
use std::{env, fs::File};

use whatsvault_core::sources::whatsapp_export_zip::{
    classify_whatsapp_export_attachment, import_whatsapp_export_zip,
    read_whatsapp_export_attachment, read_whatsapp_export_attachments,
    DEFAULT_WHATSAPP_EXPORT_IMPORT_MAX_MESSAGES,
};
use whatsvault_core::{AttachmentKind, ImportIssueCode, SourceKind};
use zip::write::SimpleFileOptions;

fn synthetic_zip(entries: &[(&str, &[u8])]) -> Cursor<Vec<u8>> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

        for (name, bytes) in entries {
            writer.start_file(name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }

        writer.finish().unwrap();
    }
    cursor.set_position(0);
    cursor
}

#[test]
fn imports_bracketed_ios_export_with_media_reference() {
    let transcript = concat!(
        "[01/02/2026, 09:15:00] Ana: hello\n",
        "[01/02/2026, 09:16:00] You: 00000001-PHOTO-2026-01-02-09-16-00.jpg\n"
    );
    let archive = synthetic_zip(&[
        ("_chat.txt", transcript.as_bytes()),
        (
            "00000001-PHOTO-2026-01-02-09-16-00.jpg",
            b"fake image bytes",
        ),
    ]);

    let imported = import_whatsapp_export_zip(archive).unwrap();

    assert_eq!(imported.source_kind, SourceKind::WhatsappExportZip);
    assert_eq!(imported.transcript_name.as_deref(), Some("_chat.txt"));
    assert_eq!(imported.messages.len(), 2);
    assert_eq!(imported.attachments.len(), 1);
    assert_eq!(imported.attachments[0].kind, AttachmentKind::Photo);
    assert_eq!(imported.messages[0].sender.as_deref(), Some("Ana"));
    assert_eq!(imported.messages[0].body, "hello");
    assert_eq!(
        imported.messages[1].attachment_ids,
        vec![imported.attachments[0].id.clone()]
    );
    assert!(imported.issues.is_empty());
}

#[test]
fn supports_dash_timestamp_exports_and_multiline_messages() {
    let transcript = concat!(
        "02/01/2026, 9:15 AM - Ana: first line\n",
        "continued line\n",
        "02/01/2026, 9:17 AM - You: reply\n"
    );
    let archive = synthetic_zip(&[("_chat.txt", transcript.as_bytes())]);

    let imported = import_whatsapp_export_zip(archive).unwrap();

    assert_eq!(imported.messages.len(), 2);
    assert_eq!(imported.messages[0].body, "first line\ncontinued line");
    assert_eq!(imported.messages[1].sender.as_deref(), Some("You"));
    assert!(imported.issues.is_empty());
}

#[test]
fn reports_continuation_without_message_as_structured_issue() {
    let archive = synthetic_zip(&[("_chat.txt", b"orphan continuation\n")]);

    let imported = import_whatsapp_export_zip(archive).unwrap();

    assert!(imported.messages.is_empty());
    assert_eq!(imported.issues.len(), 1);
    assert_eq!(
        imported.issues[0].code,
        ImportIssueCode::ContinuationWithoutMessage
    );
}

#[test]
fn imports_large_export_zip_as_bounded_recent_window() {
    let message_count = DEFAULT_WHATSAPP_EXPORT_IMPORT_MAX_MESSAGES + 3;
    let mut transcript = String::new();
    for index in 1..=message_count {
        transcript.push_str(&format!(
            "[01/02/2026, 09:15:{:02}] Ana: message {index}\n",
            index % 60
        ));
    }
    let archive = synthetic_zip(&[("_chat.txt", transcript.as_bytes())]);

    let imported = import_whatsapp_export_zip(archive).unwrap();

    assert_eq!(
        imported.messages.len(),
        DEFAULT_WHATSAPP_EXPORT_IMPORT_MAX_MESSAGES
    );
    assert_eq!(imported.messages[0].body, "message 4");
    assert_eq!(
        imported.messages.last().unwrap().body,
        format!("message {message_count}")
    );
    assert_eq!(
        imported.issues.last().unwrap().code,
        ImportIssueCode::MessageWindowTruncated
    );
}

#[test]
fn fails_when_archive_has_no_transcript() {
    let archive = synthetic_zip(&[("00000001-PHOTO-2026-01-02-09-16-00.jpg", b"fake")]);

    let err = import_whatsapp_export_zip(archive).unwrap_err();

    assert!(err.to_string().contains("transcript"));
}

#[test]
fn classifies_whatsapp_export_media_names() {
    assert_eq!(
        classify_whatsapp_export_attachment("00000001-AUDIO-2026-01-02-09-16-00.opus"),
        AttachmentKind::Audio
    );
    assert_eq!(
        classify_whatsapp_export_attachment("00000002-PHOTO-2026-01-02-09-16-00.jpg"),
        AttachmentKind::Photo
    );
    assert_eq!(
        classify_whatsapp_export_attachment("00000003-STICKER-2026-01-02-09-16-00.webp"),
        AttachmentKind::Sticker
    );
    assert_eq!(
        classify_whatsapp_export_attachment("00000004-VIDEO-2026-01-02-09-16-00.mp4"),
        AttachmentKind::Video
    );
    assert_eq!(
        classify_whatsapp_export_attachment("00000005-GIF-2026-01-02-09-16-00.mp4"),
        AttachmentKind::Gif
    );
}

#[test]
fn reads_attachment_bytes_by_normalized_archive_path() {
    let archive = synthetic_zip(&[
        ("_chat.txt", b"[01/02/2026, 09:15:00] Ana: photo\n"),
        (
            "Media\\00000001-PHOTO-2026-01-02-09-16-00.jpg",
            b"fake image bytes",
        ),
    ]);

    let payload = read_whatsapp_export_attachment(
        archive,
        "Media/00000001-PHOTO-2026-01-02-09-16-00.jpg",
        1024,
    )
    .unwrap()
    .unwrap();

    assert_eq!(payload.filename, "00000001-PHOTO-2026-01-02-09-16-00.jpg");
    assert_eq!(payload.kind, AttachmentKind::Photo);
    assert_eq!(payload.size_bytes, 16);
    assert_eq!(payload.bytes, b"fake image bytes");
}

#[test]
fn skips_attachment_payloads_over_size_limit() {
    let archive = synthetic_zip(&[
        ("_chat.txt", b"[01/02/2026, 09:15:00] Ana: photo\n"),
        (
            "00000001-PHOTO-2026-01-02-09-16-00.jpg",
            b"fake image bytes",
        ),
    ]);

    let payload =
        read_whatsapp_export_attachment(archive, "00000001-PHOTO-2026-01-02-09-16-00.jpg", 4)
            .unwrap();

    assert!(payload.is_none());
}

#[test]
fn reads_multiple_attachment_payloads_by_normalized_archive_path() {
    let archive = synthetic_zip(&[
        ("_chat.txt", b"[01/02/2026, 09:15:00] Ana: media\n"),
        (
            "Media\\00000001-PHOTO-2026-01-02-09-16-00.jpg",
            b"fake image bytes",
        ),
        (
            "Media/00000002-AUDIO-2026-01-02-09-17-00.opus",
            b"fake audio bytes",
        ),
        (
            "Media/00000003-VIDEO-2026-01-02-09-18-00.mp4",
            b"this payload is too large for this test cap",
        ),
    ]);

    let payloads = read_whatsapp_export_attachments(
        archive,
        [
            "Media/00000001-PHOTO-2026-01-02-09-16-00.jpg",
            "Media\\00000002-AUDIO-2026-01-02-09-17-00.opus",
            "Media/00000003-VIDEO-2026-01-02-09-18-00.mp4",
        ],
        16,
    )
    .unwrap();

    assert_eq!(payloads.len(), 2);
    assert_eq!(
        payloads["Media/00000001-PHOTO-2026-01-02-09-16-00.jpg"].kind,
        AttachmentKind::Photo
    );
    assert_eq!(
        payloads["Media/00000002-AUDIO-2026-01-02-09-17-00.opus"].bytes,
        b"fake audio bytes"
    );
    assert!(!payloads.contains_key("Media/00000003-VIDEO-2026-01-02-09-18-00.mp4"));
}

#[test]
#[ignore = "requires WHATSVAULT_PRIVATE_EXPORT_ZIP to point at a private local WhatsApp export"]
fn imports_private_export_zip_without_printing_chat_content() {
    let path = env::var("WHATSVAULT_PRIVATE_EXPORT_ZIP")
        .expect("WHATSVAULT_PRIVATE_EXPORT_ZIP must point at a private local ZIP");
    let file = File::open(path).expect("private WhatsApp export ZIP should be readable");

    let imported = import_whatsapp_export_zip(file).unwrap();

    assert_eq!(imported.source_kind, SourceKind::WhatsappExportZip);
    assert!(!imported.messages.is_empty());
    assert!(imported.messages.len() <= DEFAULT_WHATSAPP_EXPORT_IMPORT_MAX_MESSAGES);
    assert!(imported.transcript_name.is_some());
}
