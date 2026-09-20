use std::{
    collections::{HashMap, HashSet, VecDeque},
    io::{BufRead, BufReader, Read, Seek},
    path::Path,
};

use thiserror::Error;
use zip::ZipArchive;

use crate::{
    media::{attachment_kind_from_filename, attachment_media_type},
    Attachment, AttachmentKind, ChatImport, ImportIssue, ImportIssueCode, Message,
    MessageTimestamp, SourceKind,
};

#[derive(Debug, Error)]
pub enum ExportZipError {
    #[error("ZIP read failed: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("I/O failed while reading ZIP: {0}")]
    Io(#[from] std::io::Error),
    #[error("WhatsApp export ZIP does not contain a transcript file")]
    NoTranscript,
    #[error("WhatsApp export ZIP contains multiple transcript files: {0:?}")]
    MultipleTranscripts(Vec<String>),
    #[error("A transcript line or message exceeds the 1 MiB text limit")]
    MessageTooLarge,
}

pub type Result<T> = std::result::Result<T, ExportZipError>;

pub const DEFAULT_WHATSAPP_EXPORT_IMPORT_MAX_MESSAGES: usize = 2_000;
pub const WHATSAPP_EXPORT_MAX_MESSAGE_BYTES: usize = 1024 * 1024;
pub const WHATSAPP_EXPORT_RECENT_TEXT_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportAttachmentPayload {
    pub filename: String,
    pub kind: AttachmentKind,
    pub size_bytes: u64,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WhatsappExportImportOptions {
    pub max_messages: Option<usize>,
}

impl WhatsappExportImportOptions {
    pub fn recent(max_messages: usize) -> Self {
        Self {
            max_messages: Some(max_messages),
        }
    }

    pub fn all_messages() -> Self {
        Self { max_messages: None }
    }
}

impl Default for WhatsappExportImportOptions {
    fn default() -> Self {
        Self::recent(DEFAULT_WHATSAPP_EXPORT_IMPORT_MAX_MESSAGES)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhatsappExportImportResult {
    pub imported: ChatImport,
    pub skipped_message_count: usize,
}

pub fn import_whatsapp_export_zip<R>(_reader: R) -> Result<ChatImport>
where
    R: Read + Seek,
{
    Ok(
        import_whatsapp_export_zip_with_options(_reader, WhatsappExportImportOptions::default())?
            .imported,
    )
}

pub fn import_whatsapp_export_zip_with_options<R>(
    _reader: R,
    options: WhatsappExportImportOptions,
) -> Result<WhatsappExportImportResult>
where
    R: Read + Seek,
{
    let mut archive = ZipArchive::new(_reader)?;
    let (transcript_index, transcript_name) = find_transcript(&mut archive)?;
    let attachments = collect_attachments(&mut archive, &transcript_name);
    let transcript_file = archive.by_index(transcript_index)?;
    let mut result = parse_transcript(transcript_file, &attachments, options)?;
    result.imported.transcript_name = Some(transcript_name);
    Ok(result)
}

pub fn read_whatsapp_export_attachment<R>(
    reader: R,
    archive_path: &str,
    max_size_bytes: u64,
) -> Result<Option<ExportAttachmentPayload>>
where
    R: Read + Seek,
{
    let Some(requested_archive_path) = normalized_archive_name(archive_path) else {
        return Ok(None);
    };
    let mut payloads = read_whatsapp_export_attachments(reader, [archive_path], max_size_bytes)?;

    Ok(payloads.remove(&requested_archive_path))
}

pub fn read_whatsapp_export_attachments<R, I, S>(
    reader: R,
    archive_paths: I,
    max_size_bytes: u64,
) -> Result<HashMap<String, ExportAttachmentPayload>>
where
    R: Read + Seek,
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut requested_archive_paths: HashSet<String> = archive_paths
        .into_iter()
        .filter_map(|path| normalized_archive_name(path.as_ref()))
        .collect();
    if requested_archive_paths.is_empty() {
        return Ok(HashMap::new());
    }

    let mut archive = ZipArchive::new(reader)?;
    let mut payloads = HashMap::new();

    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        if file.is_dir() {
            continue;
        }

        let Some(current_archive_path) = normalized_archive_name(file.name()) else {
            continue;
        };
        if !requested_archive_paths.contains(&current_archive_path) {
            continue;
        }
        if file.size() > max_size_bytes {
            requested_archive_paths.remove(&current_archive_path);
            if requested_archive_paths.is_empty() {
                break;
            }
            continue;
        }

        let filename = Path::new(&current_archive_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&current_archive_path)
            .to_owned();
        let mut bytes = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut bytes)?;

        payloads.insert(
            current_archive_path.clone(),
            ExportAttachmentPayload {
                kind: classify_whatsapp_export_attachment(&filename),
                filename,
                size_bytes: file.size(),
                bytes,
            },
        );
        requested_archive_paths.remove(&current_archive_path);
        if requested_archive_paths.is_empty() {
            break;
        }
    }

    Ok(payloads)
}

pub fn classify_whatsapp_export_attachment(filename: &str) -> AttachmentKind {
    let basename = Path::new(filename)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(filename);
    let upper = basename.to_ascii_uppercase();
    if upper.contains("-GIF-") {
        return AttachmentKind::Gif;
    }
    if upper.contains("-AUDIO-") {
        return AttachmentKind::Audio;
    }
    if upper.contains("-PHOTO-") {
        return AttachmentKind::Photo;
    }
    if upper.contains("-STICKER-") {
        return AttachmentKind::Sticker;
    }
    if upper.contains("-VIDEO-") {
        return AttachmentKind::Video;
    }

    attachment_kind_from_filename(basename)
}

fn find_transcript<R>(archive: &mut ZipArchive<R>) -> Result<(usize, String)>
where
    R: Read + Seek,
{
    let mut transcript_names = Vec::new();

    for index in 0..archive.len() {
        let file = archive.by_index(index)?;
        if file.is_dir() {
            continue;
        }

        let Some(name) = normalized_archive_name(file.name()) else {
            continue;
        };

        if is_transcript_name(&name) {
            transcript_names.push((index, name));
        }
    }

    if transcript_names.is_empty() {
        return Err(ExportZipError::NoTranscript);
    }

    if transcript_names.len() == 1 {
        return Ok(transcript_names.remove(0));
    }

    let canonical: Vec<usize> = transcript_names
        .iter()
        .enumerate()
        .filter(|(_, (_, name))| {
            Path::new(name)
                .file_name()
                .is_some_and(|file| file == "_chat.txt")
        })
        .map(|(index, _)| index)
        .collect();
    if canonical.len() == 1 {
        return Ok(transcript_names.remove(canonical[0]));
    }

    Err(ExportZipError::MultipleTranscripts(
        transcript_names.into_iter().map(|(_, name)| name).collect(),
    ))
}

fn collect_attachments<R>(archive: &mut ZipArchive<R>, transcript_name: &str) -> Vec<Attachment>
where
    R: Read + Seek,
{
    let mut attachments = Vec::new();

    for index in 0..archive.len() {
        let Ok(file) = archive.by_index(index) else {
            continue;
        };
        if file.is_dir() {
            continue;
        }

        let Some(archive_path) = normalized_archive_name(file.name()) else {
            continue;
        };
        if archive_path == transcript_name {
            continue;
        }

        let filename = Path::new(&archive_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&archive_path)
            .to_owned();
        let id = format!("export-attachment-{index:08}");
        let kind = classify_whatsapp_export_attachment(&filename);

        attachments.push(Attachment {
            id,
            archive_path,
            filename,
            kind,
            size_bytes: file.size(),
        });
    }

    attachments
}

fn parse_transcript<R>(
    reader: R,
    attachments: &[Attachment],
    options: WhatsappExportImportOptions,
) -> Result<WhatsappExportImportResult>
where
    R: Read,
{
    let mut messages = VecDeque::new();
    let mut issues = Vec::new();
    let mut line = Vec::new();
    let mut reader = BufReader::new(reader);
    let mut total_message_count = 0_usize;
    let mut skipped_message_count = 0_usize;
    let mut retained_text_bytes = 0_usize;

    loop {
        line.clear();
        if reader
            .by_ref()
            .take((WHATSAPP_EXPORT_MAX_MESSAGE_BYTES + 1) as u64)
            .read_until(b'\n', &mut line)?
            == 0
        {
            break;
        }
        if line.len() > WHATSAPP_EXPORT_MAX_MESSAGE_BYTES {
            return Err(ExportZipError::MessageTooLarge);
        }

        let line = String::from_utf8_lossy(&line);
        let line = line.trim_end_matches(['\r', '\n']);
        let line = normalize_transcript_line(line);
        // Invalid UTF-8 replacement can expand the decoded text beyond the input size.
        if line.len() > WHATSAPP_EXPORT_MAX_MESSAGE_BYTES {
            return Err(ExportZipError::MessageTooLarge);
        }
        if line.is_empty() {
            continue;
        }

        if let Some(parsed) = parse_message_line(&line, total_message_count) {
            total_message_count = total_message_count.saturating_add(1);
            retained_text_bytes += message_text_bytes(&parsed);
            messages.push_back(parsed);
            trim_message_window(
                &mut messages,
                options.max_messages,
                &mut retained_text_bytes,
                &mut skipped_message_count,
            );
            continue;
        }

        if let Some(last) = messages.back_mut() {
            let added_bytes = line.len() + 1;
            if message_text_bytes(last) + added_bytes > WHATSAPP_EXPORT_MAX_MESSAGE_BYTES {
                return Err(ExportZipError::MessageTooLarge);
            }
            last.body.push('\n');
            last.body.push_str(&line);
            retained_text_bytes += added_bytes;
            trim_message_window(
                &mut messages,
                options.max_messages,
                &mut retained_text_bytes,
                &mut skipped_message_count,
            );
        } else if total_message_count == 0 && issues.is_empty() {
            issues.push(ImportIssue {
                code: ImportIssueCode::ContinuationWithoutMessage,
                message: "Transcript contains a continuation line before the first message"
                    .to_owned(),
            });
        }
    }

    let mut messages = Vec::from(messages);
    resolve_message_attachments(&mut messages, attachments, &mut issues);
    if skipped_message_count > 0 {
        issues.push(ImportIssue {
            code: ImportIssueCode::MessageWindowTruncated,
            message: format!(
                "Only the latest {} messages were loaded; older messages were skipped for performance",
                messages.len()
            ),
        });
    }
    let attachments = referenced_attachments(&messages, attachments);

    Ok(WhatsappExportImportResult {
        imported: ChatImport {
            source_kind: SourceKind::WhatsappExportZip,
            transcript_name: None,
            messages,
            attachments,
            issues,
        },
        skipped_message_count,
    })
}

fn message_text_bytes(message: &Message) -> usize {
    message.body.len()
        + message.timestamp.raw.len()
        + message.sender.as_ref().map_or(0, String::len)
}

fn trim_message_window(
    messages: &mut VecDeque<Message>,
    max_messages: Option<usize>,
    retained_text_bytes: &mut usize,
    skipped_message_count: &mut usize,
) {
    let Some(max_messages) = max_messages else {
        return;
    };

    while messages.len() > max_messages || *retained_text_bytes > WHATSAPP_EXPORT_RECENT_TEXT_BYTES
    {
        let Some(removed) = messages.pop_front() else {
            break;
        };
        *retained_text_bytes -= message_text_bytes(&removed);
        *skipped_message_count = skipped_message_count.saturating_add(1);
    }
}

fn parse_message_line(line: &str, index: usize) -> Option<Message> {
    if let Some((timestamp, rest)) = parse_bracketed_header(line) {
        return Some(message_from_header(index, timestamp, rest));
    }

    if let Some((timestamp, rest)) = parse_dash_header(line) {
        return Some(message_from_header(index, timestamp, rest));
    }

    None
}

fn message_from_header(index: usize, timestamp: &str, rest: &str) -> Message {
    let (sender, body) = split_sender(rest);

    Message {
        id: format!("export-message-{index:08}"),
        timestamp: MessageTimestamp {
            raw: timestamp.trim().to_owned(),
        },
        sender: sender.map(str::to_owned),
        body: body.trim().to_owned(),
        attachment_ids: Vec::new(),
    }
}

fn parse_bracketed_header(line: &str) -> Option<(&str, &str)> {
    let line = line.strip_prefix('[')?;
    let (timestamp, rest) = line.split_once("] ")?;
    if !looks_like_timestamp(timestamp) {
        return None;
    }
    Some((timestamp, rest))
}

fn parse_dash_header(line: &str) -> Option<(&str, &str)> {
    let (timestamp, rest) = line.split_once(" - ")?;
    if !looks_like_timestamp(timestamp) {
        return None;
    }
    Some((timestamp, rest))
}

fn split_sender(rest: &str) -> (Option<&str>, &str) {
    let Some((sender, body)) = rest.split_once(": ") else {
        return (None, rest);
    };

    if sender.trim().is_empty() {
        return (None, rest);
    }

    (Some(sender.trim()), body)
}

fn resolve_message_attachments(
    messages: &mut [Message],
    attachments: &[Attachment],
    issues: &mut Vec<ImportIssue>,
) {
    let by_filename: HashMap<&str, &Attachment> = attachments
        .iter()
        .map(|attachment| (attachment.filename.as_str(), attachment))
        .collect();
    let mut has_missing_attachment = false;

    for message in messages {
        let referenced_filenames = extract_media_like_tokens(&message.body);
        for filename in referenced_filenames {
            if let Some(attachment) = by_filename.get(filename.as_str()) {
                if !message.attachment_ids.contains(&attachment.id) {
                    message.attachment_ids.push(attachment.id.clone());
                }
            } else {
                has_missing_attachment = true;
            }
        }
    }
    if has_missing_attachment {
        issues.push(ImportIssue {
            code: ImportIssueCode::MissingAttachmentReference,
            message: "Transcript references media that is not present in the archive".to_owned(),
        });
    }
}

fn referenced_attachments(messages: &[Message], attachments: &[Attachment]) -> Vec<Attachment> {
    let referenced_ids: HashSet<&str> = messages
        .iter()
        .flat_map(|message| message.attachment_ids.iter().map(String::as_str))
        .collect();

    attachments
        .iter()
        .filter(|attachment| referenced_ids.contains(attachment.id.as_str()))
        .cloned()
        .collect()
}

fn extract_media_like_tokens(text: &str) -> Vec<String> {
    let mut filenames = Vec::new();
    for line in text.lines() {
        if let Some(filename) = line.trim().strip_suffix(" (file attached)") {
            filenames.push(filename.trim().to_owned());
            continue;
        }
        let mut remainder = line;
        while let Some((before, attached)) = remainder.split_once("<attached:") {
            let Some((filename, after)) = attached.split_once('>') else {
                break;
            };
            filenames.extend(extract_plain_media_tokens(before));
            if !filename.trim().is_empty() {
                filenames.push(filename.trim().to_owned());
            }
            remainder = after;
        }
        filenames.extend(extract_plain_media_tokens(remainder));
    }
    filenames
}

fn extract_plain_media_tokens(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter_map(|token| {
            let cleaned = token
                .trim_matches(|character: char| {
                    matches!(
                        character,
                        '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | ';' | ':'
                    )
                })
                .trim_matches('"')
                .trim_matches('\'')
                .trim_matches('\u{200e}')
                .trim_matches('\u{200f}');

            if has_media_extension(cleaned) {
                Some(cleaned.to_owned())
            } else {
                None
            }
        })
        .collect()
}

fn has_media_extension(filename: &str) -> bool {
    attachment_media_type(attachment_kind_from_filename(filename), filename).is_some()
}

fn is_transcript_name(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"))
}

fn normalized_archive_name(name: &str) -> Option<String> {
    let normalized = name.replace('\\', "/");
    if normalized.starts_with('/') || normalized.split('/').any(|part| part == "..") {
        return None;
    }
    Some(normalized)
}

fn normalize_transcript_line(line: &str) -> String {
    line.trim()
        .trim_start_matches('\u{feff}')
        .trim_start_matches('\u{200e}')
        .trim_start_matches('\u{200f}')
        .to_owned()
}

fn looks_like_timestamp(value: &str) -> bool {
    let value = value.trim();
    let Some((date, time)) = value.split_once(',').or_else(|| value.split_once(' ')) else {
        return false;
    };
    let date_parts: Vec<&str> = date.split(['/', '.', '-']).collect();
    if date_parts.len() != 3
        || !date_parts.iter().all(|part| {
            (1..=4).contains(&part.len()) && part.bytes().all(|byte| byte.is_ascii_digit())
        })
    {
        return false;
    }
    // Keep the raw locale-specific timestamp. Only recognize its numeric date and
    // clock shape here; interpreting date order belongs to presentation.
    let clock = time
        .trim()
        .split(|c: char| !c.is_ascii_digit() && c != ':')
        .next()
        .unwrap_or_default();
    let clock_parts: Vec<&str> = clock.split(':').collect();
    (2..=3).contains(&clock_parts.len())
        && clock_parts.iter().enumerate().all(|(index, part)| {
            let valid_width = if index == 0 {
                (1..=2).contains(&part.len())
            } else {
                part.len() == 2
            };
            valid_width
                && part
                    .parse::<u8>()
                    .is_ok_and(|number| number <= if index == 0 { 23 } else { 59 })
        })
}
