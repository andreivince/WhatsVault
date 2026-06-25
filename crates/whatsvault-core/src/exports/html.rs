use std::collections::HashMap;

use crate::{media::attachment_media_type, Attachment, AttachmentKind, ChatImport, Message};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HtmlExportOptions {
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmbeddedAttachment {
    pub attachment_id: String,
    pub media_type: String,
    pub base64_data: String,
}

pub fn build_chat_html_export(
    imported: &ChatImport,
    options: &HtmlExportOptions,
    embedded_attachments: &[EmbeddedAttachment],
) -> String {
    let embedded_by_id = embedded_attachments
        .iter()
        .map(|attachment| (attachment.attachment_id.as_str(), attachment))
        .collect::<HashMap<_, _>>();
    let attachments_by_id = imported
        .attachments
        .iter()
        .map(|attachment| (attachment.id.as_str(), attachment))
        .collect::<HashMap<_, _>>();
    let title = escape_html(&options.title);

    let mut html = String::new();
    html.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str(&format!("<title>{title}</title>\n"));
    html.push_str("<style>\n");
    html.push_str(EXPORT_CSS);
    html.push_str("</style>\n</head>\n<body>\n<main class=\"export-shell\">\n");
    html.push_str("<header class=\"export-header\">\n");
    html.push_str(&format!("<h1>{title}</h1>\n"));
    html.push_str(&format!(
        "<p>{} messages · {} media files · exported by WhatsVault</p>\n",
        imported.messages.len(),
        imported.attachments.len()
    ));
    html.push_str("</header>\n<section class=\"timeline\">\n");

    for message in &imported.messages {
        render_message(&mut html, message, &attachments_by_id, &embedded_by_id);
    }

    html.push_str("</section>\n</main>\n</body>\n</html>\n");
    html
}

fn render_message(
    html: &mut String,
    message: &Message,
    attachments_by_id: &HashMap<&str, &Attachment>,
    embedded_by_id: &HashMap<&str, &EmbeddedAttachment>,
) {
    let direction = if is_outgoing_message(message) {
        "outgoing"
    } else {
        "incoming"
    };

    html.push_str(&format!("<article class=\"message-row {direction}\">\n"));
    html.push_str("<div class=\"message-bubble\">\n");

    if !is_outgoing_message(message) {
        if let Some(sender) = message.sender.as_ref().filter(|sender| !sender.is_empty()) {
            html.push_str(&format!(
                "<div class=\"message-sender\">{}</div>\n",
                escape_html(sender)
            ));
        }
    }

    for attachment_id in &message.attachment_ids {
        if let Some(attachment) = attachments_by_id.get(attachment_id.as_str()) {
            render_attachment(
                html,
                attachment,
                embedded_by_id.get(attachment_id.as_str()).copied(),
            );
        }
    }

    if !message.body.is_empty() {
        html.push_str(&format!(
            "<p class=\"message-body\">{}</p>\n",
            escape_message_body(&message.body)
        ));
    }

    html.push_str(&format!(
        "<time class=\"message-time\">{}</time>\n",
        escape_html(&message.timestamp.raw)
    ));
    html.push_str("</div>\n</article>\n");
}

fn render_attachment(
    html: &mut String,
    attachment: &Attachment,
    embedded_attachment: Option<&EmbeddedAttachment>,
) {
    let filename = escape_html(&attachment.filename);

    if let Some(embedded_attachment) = embedded_attachment {
        let data_url = format!(
            "data:{};base64,{}",
            embedded_attachment.media_type, embedded_attachment.base64_data
        );

        match attachment.kind {
            AttachmentKind::Gif | AttachmentKind::Photo | AttachmentKind::Sticker => {
                html.push_str(&format!(
                    "<figure class=\"attachment\"><img src=\"{}\" alt=\"{}\"><figcaption>{}</figcaption></figure>\n",
                    escape_html_attribute(&data_url),
                    escape_html_attribute(&attachment.filename),
                    filename
                ));
            }
            AttachmentKind::Audio => {
                html.push_str(&format!(
                    "<figure class=\"attachment\"><audio controls src=\"{}\"></audio><figcaption>{}</figcaption></figure>\n",
                    escape_html_attribute(&data_url),
                    filename
                ));
            }
            AttachmentKind::Video => {
                html.push_str(&format!(
                    "<figure class=\"attachment\"><video controls src=\"{}\"></video><figcaption>{}</figcaption></figure>\n",
                    escape_html_attribute(&data_url),
                    filename
                ));
            }
            AttachmentKind::Unknown => {
                html.push_str(&format!(
                    "<p class=\"attachment-file\"><a href=\"{}\" download=\"{}\">{}</a></p>\n",
                    escape_html_attribute(&data_url),
                    escape_html_attribute(&attachment.filename),
                    filename
                ));
            }
        }
        return;
    }

    let media_hint = attachment_media_type(attachment.kind, &attachment.filename)
        .unwrap_or("attachment")
        .to_owned();
    html.push_str(&format!(
        "<p class=\"attachment-missing\">{} · {} not embedded</p>\n",
        filename,
        escape_html(&media_hint)
    ));
}

fn is_outgoing_message(message: &Message) -> bool {
    message
        .sender
        .as_deref()
        .map(|sender| {
            let normalized = sender.trim().to_ascii_lowercase();
            normalized == "you"
                || normalized == "me"
                || normalized == "voce"
                || normalized == "você"
        })
        .unwrap_or(false)
}

fn escape_message_body(value: &str) -> String {
    escape_html(value).replace('\n', "<br>")
}

fn escape_html_attribute(value: &str) -> String {
    escape_html(value)
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

const EXPORT_CSS: &str = r#"
:root {
  color-scheme: light;
  --page: #efeae2;
  --ink: #17201b;
  --muted: #65716b;
  --incoming: #ffffff;
  --outgoing: #d7fdd2;
  --accent: #008b4f;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  color: var(--ink);
  background:
    radial-gradient(circle at 20px 20px, rgba(120, 96, 66, 0.14) 0 1.5px, transparent 2px),
    linear-gradient(45deg, transparent 0 48px, rgba(120, 96, 66, 0.06) 49px 50px, transparent 51px),
    var(--page);
  background-size: 88px 88px, 122px 122px, auto;
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}
.export-shell {
  width: min(980px, 100%);
  margin: 0 auto;
  padding: 24px clamp(14px, 4vw, 40px) 56px;
}
.export-header {
  position: sticky;
  top: 0;
  z-index: 1;
  margin: -24px calc(clamp(14px, 4vw, 40px) * -1) 24px;
  padding: 18px clamp(14px, 4vw, 40px);
  background: rgba(251, 251, 250, 0.96);
  border-bottom: 1px solid rgba(0, 0, 0, 0.12);
  backdrop-filter: blur(12px);
}
.export-header h1 {
  margin: 0 0 4px;
  font-size: clamp(22px, 4vw, 32px);
  line-height: 1.1;
}
.export-header p {
  margin: 0;
  color: var(--muted);
  font-weight: 700;
}
.timeline {
  display: grid;
  gap: 5px;
}
.message-row {
  display: flex;
  width: 100%;
}
.message-row.incoming { justify-content: flex-start; }
.message-row.outgoing { justify-content: flex-end; }
.message-bubble {
  max-width: min(680px, 82%);
  padding: 8px 11px;
  border-radius: 10px;
  box-shadow: 0 1px 2px rgba(18, 20, 19, 0.14);
  font-size: 16px;
  line-height: 1.32;
}
.incoming .message-bubble {
  background: var(--incoming);
  border-top-left-radius: 3px;
}
.outgoing .message-bubble {
  background: var(--outgoing);
  border-top-right-radius: 3px;
}
.message-sender {
  margin-bottom: 3px;
  color: var(--accent);
  font-size: 13px;
  font-weight: 800;
}
.message-body {
  margin: 0;
  overflow-wrap: anywhere;
}
.message-time {
  display: block;
  margin-top: 5px;
  color: var(--muted);
  font-size: 12px;
  font-weight: 700;
  text-align: right;
}
.attachment {
  max-width: min(420px, 72vw);
  margin: 0 0 7px;
}
.attachment img,
.attachment video {
  display: block;
  max-width: 100%;
  max-height: 560px;
  border-radius: 8px;
}
.attachment audio {
  width: min(360px, 70vw);
}
.attachment figcaption,
.attachment-missing,
.attachment-file {
  margin: 5px 0 0;
  color: var(--muted);
  font-size: 12px;
  font-weight: 700;
}
.attachment-file a {
  color: var(--accent);
}
"#;
