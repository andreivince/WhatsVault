use whatsvault_core::exports::html::{
    build_chat_html_export, EmbeddedAttachment, HtmlExportOptions,
};
use whatsvault_core::{
    Attachment, AttachmentKind, ChatImport, ImportIssue, Message, MessageTimestamp, SourceKind,
};

fn sample_import() -> ChatImport {
    ChatImport {
        source_kind: SourceKind::WhatsappExportZip,
        transcript_name: Some("_chat.txt".to_owned()),
        messages: vec![
            Message {
                id: "message-1".to_owned(),
                timestamp: MessageTimestamp {
                    raw: "06/23/2026, 09:15:00".to_owned(),
                },
                sender: Some("Ana <script>".to_owned()),
                body: "hello <world>\nnew line".to_owned(),
                attachment_ids: Vec::new(),
            },
            Message {
                id: "message-2".to_owned(),
                timestamp: MessageTimestamp {
                    raw: "06/23/2026, 09:16:00".to_owned(),
                },
                sender: Some("You".to_owned()),
                body: "photo".to_owned(),
                attachment_ids: vec!["photo-1".to_owned()],
            },
        ],
        attachments: vec![Attachment {
            id: "photo-1".to_owned(),
            archive_path: "Media/photo.jpg".to_owned(),
            filename: "photo.jpg".to_owned(),
            kind: AttachmentKind::Photo,
            size_bytes: 3,
        }],
        issues: Vec::<ImportIssue>::new(),
    }
}

#[test]
fn exports_chat_html_with_escaped_text_and_metadata() {
    let html = build_chat_html_export(
        &sample_import(),
        &HtmlExportOptions {
            title: "Family <chat>".to_owned(),
        },
        &[],
    );

    assert!(html.contains("<title>Family &lt;chat&gt;</title>"));
    assert!(html.contains("Family &lt;chat&gt;"));
    assert!(html.contains("Ana &lt;script&gt;"));
    assert!(html.contains("hello &lt;world&gt;<br>new line"));
    assert!(html.contains("2 messages"));
    assert!(!html.contains("<script>"));
}

#[test]
fn embeds_available_media_as_data_urls_and_marks_missing_media() {
    let html = build_chat_html_export(
        &sample_import(),
        &HtmlExportOptions {
            title: "Media export".to_owned(),
        },
        &[EmbeddedAttachment {
            attachment_id: "photo-1".to_owned(),
            media_type: "image/jpeg".to_owned(),
            base64_data: "Zm9v".to_owned(),
        }],
    );

    assert!(html.contains("src=\"data:image/jpeg;base64,Zm9v\""));
    assert!(html.contains("alt=\"photo.jpg\""));
    assert!(html.contains("photo.jpg"));
}
