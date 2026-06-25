use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use rusqlite::{params, params_from_iter, Connection, OpenFlags, OptionalExtension};
use thiserror::Error;

use crate::{
    media::{attachment_display_label, attachment_kind_from_mime_or_filename},
    Attachment, Chat, ChatImport, ChatStorageSummary, ImportIssue, ImportIssueCode, Message,
    MessageTimestamp, SourceKind,
};

use super::chat_storage_display::{
    display_chat_title, filename_from_media_path, first_display_message_text, first_nonempty,
    first_readable_sender,
};

const MESSAGE_TABLE: &str = "ZWAMESSAGE";
const CHAT_TABLE: &str = "ZWACHATSESSION";
const MEDIA_ITEM_TABLE: &str = "ZWAMEDIAITEM";
const GROUP_MEMBER_TABLE: &str = "ZWAGROUPMEMBER";
const PROFILE_PUSH_NAME_TABLE: &str = "ZWAPROFILEPUSHNAME";
const SEARCHABLE_CHAT_LIST_FIELD_COUNT: usize = 1;
const SEARCHABLE_CHAT_MESSAGE_FIELD_COUNT: usize = 7;

#[derive(Debug, Error)]
pub enum ChatStorageError {
    #[error("SQLite failed while reading ChatStorage.sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

pub type Result<T> = std::result::Result<T, ChatStorageError>;

pub fn summarize_chat_storage<P>(path: P) -> Result<ChatStorageSummary>
where
    P: AsRef<Path>,
{
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    Ok(ChatStorageSummary {
        message_count: count_table_if_present(&connection, MESSAGE_TABLE)?,
        chat_count: count_table_if_present(&connection, CHAT_TABLE)?,
        media_item_count: count_table_if_present(&connection, MEDIA_ITEM_TABLE)?,
    })
}

pub fn list_chat_storage_chats<P>(path: P) -> Result<Vec<Chat>>
where
    P: AsRef<Path>,
{
    list_chat_storage_chats_with_limit(path, None)
}

pub fn list_chat_storage_chats_limited<P>(path: P, limit: usize) -> Result<Vec<Chat>>
where
    P: AsRef<Path>,
{
    list_chat_storage_chats_with_limit(path, Some(limit))
}

pub fn search_chat_storage_chats_limited<P>(path: P, query: &str, limit: usize) -> Result<Vec<Chat>>
where
    P: AsRef<Path>,
{
    let search_tokens = search_tokens(query);
    if search_tokens.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }

    list_chat_storage_chats_matching(path, Some(limit), &search_tokens)
}

fn list_chat_storage_chats_with_limit<P>(path: P, limit: Option<usize>) -> Result<Vec<Chat>>
where
    P: AsRef<Path>,
{
    list_chat_storage_chats_matching(path, limit, &[])
}

fn list_chat_storage_chats_matching<P>(
    path: P,
    limit: Option<usize>,
    search_tokens: &[String],
) -> Result<Vec<Chat>>
where
    P: AsRef<Path>,
{
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let Some(chat_schema) = table_schema(&connection, CHAT_TABLE)? else {
        return Ok(Vec::new());
    };
    if !chat_schema.has("Z_PK") {
        return Ok(Vec::new());
    }
    if limit == Some(0) {
        return Ok(Vec::new());
    }

    let message_schema = table_schema(&connection, MESSAGE_TABLE)?;
    let media_schema = table_schema(&connection, MEDIA_ITEM_TABLE)?;
    let query = chat_list_query(
        &chat_schema,
        message_schema.as_ref(),
        media_schema.as_ref(),
        limit,
        search_tokens.len(),
    );
    let mut query_params = Vec::new();
    for token in search_tokens {
        let pattern = like_contains_pattern(token);
        for _ in 0..SEARCHABLE_CHAT_LIST_FIELD_COUNT {
            query_params.push(pattern.clone());
        }
    }
    let mut statement = connection.prepare(&query)?;
    let rows = statement.query_map(params_from_iter(query_params.iter()), |row| {
        let latest_timestamp = row
            .get::<_, Option<String>>(3)?
            .map(|raw| MessageTimestamp { raw });
        let raw_title = row.get::<_, String>(1)?;

        Ok(Chat {
            id: row.get(0)?,
            title: display_chat_title(&raw_title),
            latest_message: row.get(2)?,
            latest_message_timestamp: latest_timestamp,
            message_count: nonnegative_i64_to_u64(row.get::<_, i64>(4)?),
            attachment_count: nonnegative_i64_to_u64(row.get::<_, i64>(5)?),
        })
    })?;

    let mut chats = Vec::new();
    for row in rows {
        chats.push(row?);
    }
    hydrate_latest_message_previews(
        &connection,
        message_schema.as_ref(),
        media_schema.as_ref(),
        &mut chats,
    )?;

    Ok(chats)
}

pub fn import_chat_storage_chat<P>(path: P, chat_id: &str) -> Result<ChatImport>
where
    P: AsRef<Path>,
{
    import_chat_storage_chat_with_message_limit(path, chat_id, None)
}

pub fn import_chat_storage_chat_recent<P>(
    path: P,
    chat_id: &str,
    message_limit: usize,
) -> Result<ChatImport>
where
    P: AsRef<Path>,
{
    import_chat_storage_chat_with_message_limit(path, chat_id, Some(message_limit))
}

pub fn search_chat_storage_chat_recent<P>(
    path: P,
    chat_id: &str,
    query: &str,
    limit: usize,
) -> Result<ChatImport>
where
    P: AsRef<Path>,
{
    search_chat_storage_chat_with_limit(path, chat_id, query, limit)
}

pub fn count_chat_storage_chat_messages<P>(path: P, chat_id: &str) -> Result<u64>
where
    P: AsRef<Path>,
{
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let Some(message_schema) = table_schema(&connection, MESSAGE_TABLE)? else {
        return Ok(0);
    };
    if !message_schema.has("ZCHATSESSION") {
        return Ok(0);
    }

    let count = connection.query_row(
        r#"
        SELECT COUNT(*)
        FROM ZWAMESSAGE
        WHERE ZCHATSESSION = ?1
        "#,
        params![chat_id],
        |row| row.get::<_, i64>(0),
    )?;

    Ok(nonnegative_i64_to_u64(count))
}

fn import_chat_storage_chat_with_message_limit<P>(
    path: P,
    chat_id: &str,
    message_limit: Option<usize>,
) -> Result<ChatImport>
where
    P: AsRef<Path>,
{
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let title = chat_title(&connection, chat_id)?;
    let Some(message_schema) = table_schema(&connection, MESSAGE_TABLE)? else {
        return Ok(empty_chat_import(title));
    };
    if !message_schema.has("Z_PK") || !message_schema.has("ZCHATSESSION") {
        return Ok(empty_chat_import(title));
    }
    if message_limit == Some(0) {
        return Ok(empty_chat_import(title));
    }

    let media_schema = table_schema(&connection, MEDIA_ITEM_TABLE)?;
    let group_member_schema = table_schema(&connection, GROUP_MEMBER_TABLE)?;
    let profile_push_name_schema = table_schema(&connection, PROFILE_PUSH_NAME_TABLE)?;
    let media_join = media_join_expr(&message_schema, media_schema.as_ref());
    let group_member_join = group_member_join_expr(&message_schema, group_member_schema.as_ref());
    let profile_push_name_join = profile_push_name_join_expr(
        &message_schema,
        group_member_schema.as_ref(),
        profile_push_name_schema.as_ref(),
    );
    let message_text_expr = nullable_text_expr("m", &message_schema, "ZTEXT");
    let from_jid_expr = nullable_text_expr("m", &message_schema, "ZFROMJID");
    let push_name_expr = nullable_text_expr("m", &message_schema, "ZPUSHNAME");
    let group_member_name_expr = group_member_name_expr(group_member_schema.as_ref());
    let profile_push_name_expr = profile_push_name_expr(profile_push_name_schema.as_ref());
    let message_date_expr = nullable_number_expr("m", &message_schema, "ZMESSAGEDATE");
    let is_from_me_expr = nullable_number_expr("m", &message_schema, "ZISFROMME");
    let media_pk_expr = media_column_expr(media_schema.as_ref(), "Z_PK");
    let media_path_expr = media_text_column_expr(media_schema.as_ref(), "ZMEDIALOCALPATH");
    let media_type_expr = media_text_column_expr(media_schema.as_ref(), "ZVCARDSTRING");
    let media_title_expr = media_text_column_expr(media_schema.as_ref(), "ZTITLE");
    let media_size_expr = media_column_expr(media_schema.as_ref(), "ZFILESIZE");
    let order_expr = message_order_expr(&message_schema);

    let query = format!(
        r#"
        SELECT
            m.Z_PK AS message_pk,
            {message_timestamp_expr} AS message_timestamp,
            {is_from_me_expr} AS is_from_me,
            {message_text_expr} AS message_text,
            {group_member_name_expr} AS group_member_name,
            {profile_push_name_expr} AS profile_push_name,
            {from_jid_expr} AS from_jid,
            {push_name_expr} AS push_name,
            {media_pk_expr} AS media_pk,
            {media_path_expr} AS media_path,
            {media_type_expr} AS media_type,
            {media_title_expr} AS media_title,
            {media_size_expr} AS media_size
        FROM {MESSAGE_TABLE} m
        {media_join}
        {group_member_join}
        {profile_push_name_join}
        WHERE {message_filter_expr}
        ORDER BY {order_expr}, m.Z_PK
        "#,
        message_timestamp_expr = cocoa_timestamp_expr(&message_date_expr),
        message_filter_expr = chat_message_filter_expr(&message_schema, message_limit),
    );
    let mut statement = connection.prepare(&query)?;
    let rows = statement.query_map(params![chat_id], chat_storage_message_row_from_sql)?;
    let mut message_rows = Vec::new();
    for row in rows {
        message_rows.push(row?);
    }

    Ok(chat_import_from_message_rows(
        title,
        message_rows,
        Vec::new(),
    ))
}

fn search_chat_storage_chat_with_limit<P>(
    path: P,
    chat_id: &str,
    query: &str,
    limit: usize,
) -> Result<ChatImport>
where
    P: AsRef<Path>,
{
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let title = chat_title(&connection, chat_id)?;
    let search_tokens = search_tokens(query);
    if search_tokens.is_empty() || limit == 0 {
        return Ok(empty_chat_import(title));
    }

    let Some(message_schema) = table_schema(&connection, MESSAGE_TABLE)? else {
        return Ok(empty_chat_import(title));
    };
    if !message_schema.has("Z_PK") || !message_schema.has("ZCHATSESSION") {
        return Ok(empty_chat_import(title));
    }

    let media_schema = table_schema(&connection, MEDIA_ITEM_TABLE)?;
    let group_member_schema = table_schema(&connection, GROUP_MEMBER_TABLE)?;
    let profile_push_name_schema = table_schema(&connection, PROFILE_PUSH_NAME_TABLE)?;
    let media_join = media_join_expr(&message_schema, media_schema.as_ref());
    let group_member_join = group_member_join_expr(&message_schema, group_member_schema.as_ref());
    let profile_push_name_join = profile_push_name_join_expr(
        &message_schema,
        group_member_schema.as_ref(),
        profile_push_name_schema.as_ref(),
    );
    let message_text_expr = nullable_text_expr("m", &message_schema, "ZTEXT");
    let from_jid_expr = nullable_text_expr("m", &message_schema, "ZFROMJID");
    let push_name_expr = nullable_text_expr("m", &message_schema, "ZPUSHNAME");
    let group_member_name_expr = group_member_name_expr(group_member_schema.as_ref());
    let profile_push_name_expr = profile_push_name_expr(profile_push_name_schema.as_ref());
    let message_date_expr = nullable_number_expr("m", &message_schema, "ZMESSAGEDATE");
    let is_from_me_expr = nullable_number_expr("m", &message_schema, "ZISFROMME");
    let media_pk_expr = media_column_expr(media_schema.as_ref(), "Z_PK");
    let media_path_expr = media_text_column_expr(media_schema.as_ref(), "ZMEDIALOCALPATH");
    let media_type_expr = media_text_column_expr(media_schema.as_ref(), "ZVCARDSTRING");
    let media_title_expr = media_text_column_expr(media_schema.as_ref(), "ZTITLE");
    let media_size_expr = media_column_expr(media_schema.as_ref(), "ZFILESIZE");
    let order_expr = message_order_expr(&message_schema);
    let search_expr = chat_search_filter_expr(
        &[
            message_text_expr.as_str(),
            group_member_name_expr.as_str(),
            profile_push_name_expr.as_str(),
            push_name_expr.as_str(),
            from_jid_expr.as_str(),
            media_path_expr.as_str(),
            media_title_expr.as_str(),
        ],
        search_tokens.len(),
    );
    let bounded_limit = limit.saturating_add(1);
    let query = format!(
        r#"
        SELECT
            m.Z_PK AS message_pk,
            {message_timestamp_expr} AS message_timestamp,
            {is_from_me_expr} AS is_from_me,
            {message_text_expr} AS message_text,
            {group_member_name_expr} AS group_member_name,
            {profile_push_name_expr} AS profile_push_name,
            {from_jid_expr} AS from_jid,
            {push_name_expr} AS push_name,
            {media_pk_expr} AS media_pk,
            {media_path_expr} AS media_path,
            {media_type_expr} AS media_type,
            {media_title_expr} AS media_title,
            {media_size_expr} AS media_size
        FROM {MESSAGE_TABLE} m
        {media_join}
        {group_member_join}
        {profile_push_name_join}
        WHERE m.ZCHATSESSION = ?
          AND {search_expr}
        ORDER BY {order_expr} DESC, m.Z_PK DESC
        LIMIT {bounded_limit}
        "#,
        message_timestamp_expr = cocoa_timestamp_expr(&message_date_expr),
    );
    let mut query_params = vec![chat_id.to_owned()];
    for token in &search_tokens {
        let pattern = like_contains_pattern(token);
        for _ in 0..SEARCHABLE_CHAT_MESSAGE_FIELD_COUNT {
            query_params.push(pattern.clone());
        }
    }
    let mut statement = connection.prepare(&query)?;
    let rows = statement.query_map(params_from_iter(query_params.iter()), |row| {
        chat_storage_message_row_from_sql(row)
    })?;
    let mut message_rows = Vec::new();
    for row in rows {
        message_rows.push(row?);
    }

    let is_truncated = message_rows.len() > limit;
    if is_truncated {
        message_rows.truncate(limit);
    }
    message_rows.reverse();
    let issues = if is_truncated {
        vec![ImportIssue {
            code: ImportIssueCode::SearchResultsTruncated,
            message: format!("Only the latest {limit} matching messages were loaded"),
        }]
    } else {
        Vec::new()
    };

    Ok(chat_import_from_message_rows(title, message_rows, issues))
}

fn chat_storage_message_row_from_sql(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ChatStorageMessageRow> {
    Ok(ChatStorageMessageRow {
        message_pk: row.get(0)?,
        timestamp: row.get(1)?,
        is_from_me: row.get(2)?,
        message_text: row.get(3)?,
        group_member_name: row.get(4)?,
        profile_push_name: row.get(5)?,
        from_jid: row.get(6)?,
        push_name: row.get(7)?,
        media_pk: row.get(8)?,
        media_path: row.get(9)?,
        media_type: row.get(10)?,
        media_title: row.get(11)?,
        media_size: row.get(12)?,
    })
}

fn chat_import_from_message_rows(
    title: String,
    rows: Vec<ChatStorageMessageRow>,
    issues: Vec<ImportIssue>,
) -> ChatImport {
    let mut messages = Vec::new();
    let mut attachments = Vec::new();
    let mut seen_attachment_ids = HashSet::new();

    for row in rows {
        let attachment = row.attachment();
        let attachment_ids = attachment
            .as_ref()
            .map(|attachment| vec![attachment.id.clone()])
            .unwrap_or_default();
        if let Some(attachment) = attachment {
            if seen_attachment_ids.insert(attachment.id.clone()) {
                attachments.push(attachment);
            }
        }

        messages.push(Message {
            id: format!(
                "chat-storage-message-{message_pk:08}",
                message_pk = row.message_pk
            ),
            timestamp: MessageTimestamp {
                raw: row
                    .timestamp
                    .clone()
                    .unwrap_or_else(|| "Unknown time".to_owned()),
            },
            sender: row.sender(),
            body: row.body(),
            attachment_ids,
        });
    }

    ChatImport {
        source_kind: SourceKind::IphoneBackup,
        transcript_name: Some(title),
        messages,
        attachments,
        issues,
    }
}

fn count_table_if_present(connection: &Connection, table_name: &str) -> Result<Option<u64>> {
    if !table_exists(connection, table_name)? {
        return Ok(None);
    }

    let query = format!("SELECT COUNT(*) FROM {table_name}");
    let count = connection.query_row(&query, [], |row| row.get::<_, i64>(0))?;
    Ok(Some(count as u64))
}

fn table_exists(connection: &Connection, table_name: &str) -> Result<bool> {
    let count = connection.query_row(
        r#"
        SELECT COUNT(*)
        FROM sqlite_master
        WHERE type = 'table'
          AND name = ?1
        "#,
        [table_name],
        |row| row.get::<_, i64>(0),
    )?;

    Ok(count > 0)
}

fn table_schema(connection: &Connection, table_name: &'static str) -> Result<Option<TableSchema>> {
    if !table_exists(connection, table_name)? {
        return Ok(None);
    }

    let query = format!("PRAGMA table_info({table_name})");
    let mut statement = connection.prepare(&query)?;
    let rows = statement.query_map([], |row| row.get::<_, String>(1))?;
    let mut columns = HashSet::new();

    for row in rows {
        columns.insert(row?);
    }

    Ok(Some(TableSchema {
        name: table_name,
        columns,
    }))
}

#[derive(Debug, Clone)]
struct TableSchema {
    name: &'static str,
    columns: HashSet<String>,
}

impl TableSchema {
    fn has(&self, column: &str) -> bool {
        self.columns.contains(column)
    }
}

#[derive(Debug, Clone)]
struct ChatStorageMessageRow {
    message_pk: i64,
    timestamp: Option<String>,
    is_from_me: Option<i64>,
    message_text: Option<String>,
    group_member_name: Option<String>,
    profile_push_name: Option<String>,
    from_jid: Option<String>,
    push_name: Option<String>,
    media_pk: Option<i64>,
    media_path: Option<String>,
    media_type: Option<String>,
    media_title: Option<String>,
    media_size: Option<i64>,
}

impl ChatStorageMessageRow {
    fn sender(&self) -> Option<String> {
        if self.is_from_me == Some(1) {
            return Some("You".to_owned());
        }

        first_readable_sender([
            self.group_member_name.as_deref(),
            self.profile_push_name.as_deref(),
            self.push_name.as_deref(),
            self.from_jid.as_deref(),
        ])
        .or_else(|| Some("Participant".to_owned()))
    }

    fn body(&self) -> String {
        if let Some(message_text) = first_display_message_text([self.message_text.as_deref()]) {
            return message_text;
        }

        self.media_label().unwrap_or_default()
    }

    fn media_label(&self) -> Option<String> {
        let archive_path =
            first_nonempty([self.media_path.as_deref(), self.media_title.as_deref()]);
        if self.media_pk.is_none() && archive_path.is_none() {
            return None;
        }

        let filename = archive_path
            .as_deref()
            .map(filename_from_media_path)
            .unwrap_or_default();
        let kind = attachment_kind_from_mime_or_filename(self.media_type.as_deref(), &filename);

        Some(attachment_display_label(kind).to_owned())
    }

    fn attachment(&self) -> Option<Attachment> {
        let media_pk = self.media_pk?;
        let archive_path =
            first_nonempty([self.media_path.as_deref(), self.media_title.as_deref()])?;
        let filename = filename_from_media_path(&archive_path);
        let kind = attachment_kind_from_mime_or_filename(self.media_type.as_deref(), &filename);

        Some(Attachment {
            id: format!("chat-storage-media-{media_pk:08}"),
            archive_path,
            filename,
            kind,
            size_bytes: self
                .media_size
                .map(nonnegative_i64_to_u64)
                .unwrap_or_default(),
        })
    }
}

#[derive(Debug, Clone)]
struct LatestMessagePreviewRow {
    chat_id: String,
    message_text: Option<String>,
    media_pk: Option<i64>,
    media_path: Option<String>,
    media_type: Option<String>,
    media_title: Option<String>,
}

impl LatestMessagePreviewRow {
    fn body(&self) -> Option<String> {
        if let Some(message_text) = first_display_message_text([self.message_text.as_deref()]) {
            return Some(message_text);
        }

        let archive_path =
            first_nonempty([self.media_path.as_deref(), self.media_title.as_deref()]);
        if self.media_pk.is_none() && archive_path.is_none() {
            return None;
        }

        let filename = archive_path
            .as_deref()
            .map(filename_from_media_path)
            .unwrap_or_default();
        let kind = attachment_kind_from_mime_or_filename(self.media_type.as_deref(), &filename);

        Some(attachment_display_label(kind).to_owned())
    }
}

fn hydrate_latest_message_previews(
    connection: &Connection,
    message_schema: Option<&TableSchema>,
    media_schema: Option<&TableSchema>,
    chats: &mut [Chat],
) -> Result<()> {
    let Some(message_schema) = message_schema else {
        return Ok(());
    };
    if chats.is_empty() || !message_schema.has("Z_PK") || !message_schema.has("ZCHATSESSION") {
        return Ok(());
    }

    let placeholders = (1..=chats.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let media_join = media_join_expr(message_schema, media_schema);
    let message_text_expr = nullable_text_expr("m", message_schema, "ZTEXT");
    let media_pk_expr = media_column_expr(media_schema, "Z_PK");
    let media_path_expr = media_text_column_expr(media_schema, "ZMEDIALOCALPATH");
    let media_type_expr = media_text_column_expr(media_schema, "ZVCARDSTRING");
    let media_title_expr = media_text_column_expr(media_schema, "ZTITLE");
    let order_expr = message_order_expr(message_schema);
    let query = format!(
        r#"
        SELECT
            CAST(chat_id AS TEXT) AS chat_id,
            message_text,
            media_pk,
            media_path,
            media_type,
            media_title
        FROM (
            SELECT
                m.ZCHATSESSION AS chat_id,
                {message_text_expr} AS message_text,
                {media_pk_expr} AS media_pk,
                {media_path_expr} AS media_path,
                {media_type_expr} AS media_type,
                {media_title_expr} AS media_title,
                ROW_NUMBER() OVER (
                    PARTITION BY m.ZCHATSESSION
                    ORDER BY {order_expr} DESC, m.Z_PK DESC
                ) AS row_number
            FROM {MESSAGE_TABLE} m
            {media_join}
            WHERE m.ZCHATSESSION IN ({placeholders})
        )
        WHERE row_number = 1
        "#
    );
    let chat_ids = chats
        .iter()
        .map(|chat| chat.id.as_str())
        .collect::<Vec<_>>();
    let mut statement = connection.prepare(&query)?;
    let rows = statement.query_map(params_from_iter(chat_ids), |row| {
        Ok(LatestMessagePreviewRow {
            chat_id: row.get(0)?,
            message_text: row.get(1)?,
            media_pk: row.get(2)?,
            media_path: row.get(3)?,
            media_type: row.get(4)?,
            media_title: row.get(5)?,
        })
    })?;
    let mut previews = HashMap::new();
    for row in rows {
        let row = row?;
        previews.insert(row.chat_id.clone(), row.body());
    }

    for chat in chats {
        if let Some(preview) = previews.remove(&chat.id).flatten() {
            chat.latest_message = Some(preview);
        }
    }

    Ok(())
}

fn chat_title(connection: &Connection, chat_id: &str) -> Result<String> {
    let Some(chat_schema) = table_schema(connection, CHAT_TABLE)? else {
        return Ok(format!("Chat {chat_id}"));
    };
    if !chat_schema.has("Z_PK") {
        return Ok(format!("Chat {chat_id}"));
    }

    let title_expr = coalesced_text_expr(
        "c",
        &chat_schema,
        &["ZPARTNERNAME", "ZCONTACTJID"],
        "'Imported chat'",
    );
    let query = format!(
        r#"
        SELECT {title_expr}
        FROM {CHAT_TABLE} c
        WHERE c.Z_PK = ?1
        LIMIT 1
        "#
    );

    let raw_title = connection
        .query_row(&query, params![chat_id], |row| row.get::<_, String>(0))
        .optional()?
        .unwrap_or_else(|| format!("Chat {chat_id}"));
    Ok(display_chat_title(&raw_title))
}

fn empty_chat_import(title: String) -> ChatImport {
    ChatImport {
        source_kind: SourceKind::IphoneBackup,
        transcript_name: Some(title),
        messages: Vec::new(),
        attachments: Vec::new(),
        issues: Vec::new(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ChatListQueryParts {
    with_clause: String,
    joins: String,
    latest_date_expr: String,
    message_count_expr: String,
    attachment_count_expr: String,
    sort_date_expr: String,
}

fn chat_list_query(
    chat_schema: &TableSchema,
    message_schema: Option<&TableSchema>,
    media_schema: Option<&TableSchema>,
    limit: Option<usize>,
    search_token_count: usize,
) -> String {
    let title_expr = coalesced_text_expr(
        "c",
        chat_schema,
        &["ZPARTNERNAME", "ZCONTACTJID"],
        "'Imported chat'",
    );
    let parts = chat_list_query_parts(chat_schema, message_schema, media_schema);
    let where_clause = if search_token_count == 0 {
        String::new()
    } else {
        format!(
            "WHERE {}",
            chat_search_filter_expr(&[title_expr.as_str()], search_token_count)
        )
    };

    format!(
        r#"
        {with_clause}
        SELECT
            CAST(c.Z_PK AS TEXT) AS id,
            {title_expr} AS title,
            NULL AS latest_message,
            {last_timestamp_expr} AS latest_message_timestamp,
            COALESCE({message_count_expr}, 0) AS message_count,
            COALESCE({attachment_count_expr}, 0) AS attachment_count,
            {sort_date_expr} AS sort_date
        FROM {CHAT_TABLE} c
        {joins}
        {where_clause}
        ORDER BY sort_date DESC, c.Z_PK DESC
        {limit_clause}
        "#,
        with_clause = parts.with_clause,
        joins = parts.joins,
        where_clause = where_clause,
        last_timestamp_expr = cocoa_timestamp_expr(&parts.latest_date_expr),
        message_count_expr = parts.message_count_expr,
        attachment_count_expr = parts.attachment_count_expr,
        sort_date_expr = parts.sort_date_expr,
        limit_clause = sql_limit_clause(limit),
    )
}

fn chat_list_query_parts(
    chat_schema: &TableSchema,
    message_schema: Option<&TableSchema>,
    media_schema: Option<&TableSchema>,
) -> ChatListQueryParts {
    let mut ctes = Vec::new();
    let mut joins = Vec::new();
    let mut latest_date_expr = if chat_schema.has("ZLASTMESSAGEDATE") {
        "c.ZLASTMESSAGEDATE".to_owned()
    } else {
        "NULL".to_owned()
    };
    let mut message_count_expr = if chat_schema.has("ZMESSAGECOUNTER") {
        "c.ZMESSAGECOUNTER".to_owned()
    } else {
        "0".to_owned()
    };
    let mut attachment_count_expr = "0".to_owned();

    if let Some(message_schema) = message_schema.filter(|schema| schema.has("ZCHATSESSION")) {
        let latest_message_date_expr = if message_schema.has("ZMESSAGEDATE") {
            "MAX(m.ZMESSAGEDATE)".to_owned()
        } else {
            "NULL".to_owned()
        };
        ctes.push(format!(
            r#"
            message_summary AS (
                SELECT
                    m.ZCHATSESSION AS chat_id,
                    COUNT(*) AS message_count,
                    {latest_message_date_expr} AS latest_message_date
                FROM {MESSAGE_TABLE} m
                GROUP BY m.ZCHATSESSION
            )
            "#
        ));
        joins.push("LEFT JOIN message_summary ms ON ms.chat_id = c.Z_PK".to_owned());
        message_count_expr = "ms.message_count".to_owned();
        if message_schema.has("ZMESSAGEDATE") {
            latest_date_expr = "ms.latest_message_date".to_owned();
        }

        if let Some(attachment_summary_cte) =
            chat_attachment_summary_cte(message_schema, media_schema)
        {
            ctes.push(attachment_summary_cte);
            joins.push("LEFT JOIN attachment_summary ats ON ats.chat_id = c.Z_PK".to_owned());
            attachment_count_expr = "ats.attachment_count".to_owned();
        }
    }

    ChatListQueryParts {
        with_clause: sql_with_clause(ctes),
        joins: joins.join("\n"),
        sort_date_expr: format!("COALESCE({latest_date_expr}, 0)"),
        latest_date_expr,
        message_count_expr,
        attachment_count_expr,
    }
}

fn chat_attachment_summary_cte(
    message_schema: &TableSchema,
    media_schema: Option<&TableSchema>,
) -> Option<String> {
    let media_schema = media_schema?;
    if !message_schema.has("Z_PK") || !message_schema.has("ZCHATSESSION") {
        return None;
    }

    let join_condition = media_join_condition(message_schema, media_schema)?;
    let importable_predicate = media_importable_predicate(media_schema)?;

    Some(format!(
        r#"
            attachment_summary AS (
                SELECT
                    m.ZCHATSESSION AS chat_id,
                    COUNT(*) AS attachment_count
                FROM {MESSAGE_TABLE} m
                JOIN {MEDIA_ITEM_TABLE} mi ON {join_condition}
                WHERE {importable_predicate}
                GROUP BY m.ZCHATSESSION
            )
            "#
    ))
}

fn sql_with_clause(ctes: Vec<String>) -> String {
    if ctes.is_empty() {
        String::new()
    } else {
        format!(
            "WITH {}",
            ctes.into_iter()
                .map(|cte| cte.trim().to_owned())
                .collect::<Vec<_>>()
                .join(",\n")
        )
    }
}

fn media_join_expr(message_schema: &TableSchema, media_schema: Option<&TableSchema>) -> String {
    let Some(media_schema) = media_schema else {
        return String::new();
    };

    media_join_condition(message_schema, media_schema)
        .map(|condition| format!("LEFT JOIN {} mi ON {condition}", media_schema.name))
        .unwrap_or_default()
}

fn group_member_join_expr(
    message_schema: &TableSchema,
    group_member_schema: Option<&TableSchema>,
) -> String {
    let Some(group_member_schema) = group_member_schema else {
        return String::new();
    };
    if !message_schema.has("ZGROUPMEMBER") || !group_member_schema.has("Z_PK") {
        return String::new();
    }

    format!(
        "LEFT JOIN {} gm ON m.ZGROUPMEMBER = gm.Z_PK",
        group_member_schema.name
    )
}

fn profile_push_name_join_expr(
    message_schema: &TableSchema,
    group_member_schema: Option<&TableSchema>,
    profile_push_name_schema: Option<&TableSchema>,
) -> String {
    let Some(profile_push_name_schema) = profile_push_name_schema else {
        return String::new();
    };
    if !profile_push_name_schema.has("ZJID") || !profile_push_name_schema.has("ZPUSHNAME") {
        return String::new();
    }
    let Some(sender_jid_expr) = sender_jid_expr(message_schema, group_member_schema) else {
        return String::new();
    };

    format!(
        "LEFT JOIN {} ppn ON ppn.ZJID = {sender_jid_expr}",
        profile_push_name_schema.name
    )
}

fn sender_jid_expr(
    message_schema: &TableSchema,
    group_member_schema: Option<&TableSchema>,
) -> Option<String> {
    let mut candidates = Vec::new();
    if group_member_schema.is_some_and(|schema| schema.has("ZMEMBERJID")) {
        candidates.push(text_column_expr("gm", "ZMEMBERJID"));
    }
    if message_schema.has("ZFROMJID") {
        candidates.push(text_column_expr("m", "ZFROMJID"));
    }

    match candidates.len() {
        0 => None,
        1 => candidates.into_iter().next(),
        _ => Some(format!("COALESCE({})", candidates.join(", "))),
    }
}

fn media_join_condition(
    message_schema: &TableSchema,
    media_schema: &TableSchema,
) -> Option<String> {
    if message_schema.has("ZMEDIAITEM") && media_schema.has("Z_PK") {
        return Some("m.ZMEDIAITEM = mi.Z_PK".to_owned());
    }

    if message_schema.has("Z_PK") && media_schema.has("ZMESSAGE") {
        return Some("mi.ZMESSAGE = m.Z_PK".to_owned());
    }

    None
}

fn group_member_name_expr(group_member_schema: Option<&TableSchema>) -> String {
    let Some(group_member_schema) = group_member_schema else {
        return "NULL".to_owned();
    };

    coalesced_optional_text_expr("gm", group_member_schema, &["ZCONTACTNAME", "ZFIRSTNAME"])
}

fn profile_push_name_expr(profile_push_name_schema: Option<&TableSchema>) -> String {
    profile_push_name_schema
        .filter(|schema| schema.has("ZPUSHNAME"))
        .map(|schema| nullable_text_expr("ppn", schema, "ZPUSHNAME"))
        .unwrap_or_else(|| "NULL".to_owned())
}

fn media_importable_predicate(media_schema: &TableSchema) -> Option<String> {
    let predicates: Vec<String> = ["ZMEDIALOCALPATH", "ZTITLE"]
        .into_iter()
        .filter(|column| media_schema.has(column))
        .map(|column| format!("{} IS NOT NULL", text_column_expr("mi", column)))
        .collect();

    if predicates.is_empty() {
        None
    } else {
        Some(format!("({})", predicates.join(" OR ")))
    }
}

fn message_order_expr(message_schema: &TableSchema) -> String {
    message_order_expr_for_alias(message_schema, "m")
}

fn message_order_expr_for_alias(message_schema: &TableSchema, alias: &str) -> String {
    if message_schema.has("ZSORT") {
        return format!("{alias}.ZSORT");
    }
    if message_schema.has("ZMESSAGEDATE") {
        return format!("{alias}.ZMESSAGEDATE");
    }

    format!("{alias}.Z_PK")
}

fn chat_message_filter_expr(message_schema: &TableSchema, message_limit: Option<usize>) -> String {
    let Some(limit) = message_limit else {
        return "m.ZCHATSESSION = ?1".to_owned();
    };
    let order_expr = message_order_expr_for_alias(message_schema, "scoped");

    format!(
        r#"
        m.Z_PK IN (
            SELECT scoped.Z_PK
            FROM {MESSAGE_TABLE} scoped
            WHERE scoped.ZCHATSESSION = ?1
            ORDER BY {order_expr} DESC, scoped.Z_PK DESC
            LIMIT {limit}
        )
        "#
    )
}

fn chat_search_filter_expr(searchable_exprs: &[&str], token_count: usize) -> String {
    if token_count == 0 {
        return "1 = 1".to_owned();
    }

    (0..token_count)
        .map(|_| {
            format!(
                "({})",
                searchable_exprs
                    .iter()
                    .map(|expr| format!("LOWER(COALESCE({expr}, '')) LIKE ? ESCAPE '\\'"))
                    .collect::<Vec<_>>()
                    .join(" OR ")
            )
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn sql_limit_clause(limit: Option<usize>) -> String {
    limit
        .map(|limit| format!("LIMIT {limit}"))
        .unwrap_or_default()
}

fn coalesced_text_expr(
    alias: &str,
    schema: &TableSchema,
    columns: &[&str],
    fallback: &str,
) -> String {
    let mut parts: Vec<String> = columns
        .iter()
        .filter(|column| schema.has(column))
        .map(|column| text_column_expr(alias, column))
        .collect();
    parts.push(fallback.to_owned());

    format!("COALESCE({})", parts.join(", "))
}

fn coalesced_optional_text_expr(alias: &str, schema: &TableSchema, columns: &[&str]) -> String {
    let parts: Vec<String> = columns
        .iter()
        .filter(|column| schema.has(column))
        .map(|column| text_column_expr(alias, column))
        .collect();

    match parts.len() {
        0 => "NULL".to_owned(),
        1 => parts
            .into_iter()
            .next()
            .unwrap_or_else(|| "NULL".to_owned()),
        _ => format!("COALESCE({})", parts.join(", ")),
    }
}

fn nullable_text_expr(alias: &str, schema: &TableSchema, column: &str) -> String {
    if schema.has(column) {
        text_column_expr(alias, column)
    } else {
        "NULL".to_owned()
    }
}

fn nullable_number_expr(alias: &str, schema: &TableSchema, column: &str) -> String {
    if schema.has(column) {
        format!("{alias}.{column}")
    } else {
        "NULL".to_owned()
    }
}

fn media_column_expr(media_schema: Option<&TableSchema>, column: &str) -> String {
    media_schema
        .filter(|schema| schema.has(column))
        .map(|_| format!("mi.{column}"))
        .unwrap_or_else(|| "NULL".to_owned())
}

fn media_text_column_expr(media_schema: Option<&TableSchema>, column: &str) -> String {
    media_schema
        .filter(|schema| schema.has(column))
        .map(|_| text_column_expr("mi", column))
        .unwrap_or_else(|| "NULL".to_owned())
}

fn text_column_expr(alias: &str, column: &str) -> String {
    format!("NULLIF(TRIM(CAST({alias}.{column} AS TEXT)), '')")
}

fn cocoa_timestamp_expr(seconds_expr: &str) -> String {
    format!("strftime('%m/%d/%Y, %H:%M', '2001-01-01', CAST({seconds_expr} AS TEXT) || ' seconds')")
}

fn search_tokens(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(|token| token.trim().to_lowercase())
        .filter(|token| !token.is_empty())
        .collect()
}

fn like_contains_pattern(token: &str) -> String {
    let mut pattern = String::with_capacity(token.len() + 2);
    pattern.push('%');
    for character in token.chars() {
        match character {
            '\\' | '%' | '_' => {
                pattern.push('\\');
                pattern.push(character);
            }
            _ => pattern.push(character),
        }
    }
    pattern.push('%');
    pattern
}

fn nonnegative_i64_to_u64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema(name: &'static str, columns: &[&str]) -> TableSchema {
        TableSchema {
            name,
            columns: columns.iter().map(|column| (*column).to_owned()).collect(),
        }
    }

    #[test]
    fn chat_list_query_uses_joined_aggregates_for_large_histories() {
        let chat_schema = schema(
            CHAT_TABLE,
            &[
                "Z_PK",
                "ZCONTACTJID",
                "ZPARTNERNAME",
                "ZMESSAGECOUNTER",
                "ZLASTMESSAGEDATE",
            ],
        );
        let message_schema = schema(
            MESSAGE_TABLE,
            &["Z_PK", "ZCHATSESSION", "ZMESSAGEDATE", "ZMEDIAITEM"],
        );
        let media_schema = schema(MEDIA_ITEM_TABLE, &["Z_PK", "ZMEDIALOCALPATH", "ZTITLE"]);

        let query = chat_list_query(
            &chat_schema,
            Some(&message_schema),
            Some(&media_schema),
            Some(1_000),
            0,
        );

        assert!(query.contains("WITH message_summary AS"));
        assert!(query.contains("attachment_summary AS"));
        assert!(query.contains("LEFT JOIN message_summary ms ON ms.chat_id = c.Z_PK"));
        assert!(query.contains("LEFT JOIN attachment_summary ats ON ats.chat_id = c.Z_PK"));
        assert!(query.contains("ORDER BY sort_date DESC, c.Z_PK DESC"));
        assert!(query.contains("LIMIT 1000"));
        assert!(!query.contains("(SELECT"));
        assert!(!query.contains("WHERE m.ZCHATSESSION = c.Z_PK"));
    }

    #[test]
    fn chat_list_query_falls_back_to_chat_metadata_without_message_thread_column() {
        let chat_schema = schema(
            CHAT_TABLE,
            &["Z_PK", "ZCONTACTJID", "ZMESSAGECOUNTER", "ZLASTMESSAGEDATE"],
        );
        let message_schema = schema(MESSAGE_TABLE, &["Z_PK", "ZTEXT"]);

        let query = chat_list_query(&chat_schema, Some(&message_schema), None, None, 0);

        assert!(!query.contains("WITH message_summary AS"));
        assert!(query.contains("COALESCE(c.ZMESSAGECOUNTER, 0) AS message_count"));
        assert!(query.contains("COALESCE(c.ZLASTMESSAGEDATE, 0) AS sort_date"));
    }

    #[test]
    fn chat_list_query_filters_titles_without_correlated_scans() {
        let chat_schema = schema(CHAT_TABLE, &["Z_PK", "ZCONTACTJID", "ZPARTNERNAME"]);
        let message_schema = schema(MESSAGE_TABLE, &["Z_PK", "ZCHATSESSION", "ZMESSAGEDATE"]);

        let query = chat_list_query(&chat_schema, Some(&message_schema), None, Some(200), 2);

        assert!(query.contains("WHERE (LOWER(COALESCE("));
        assert!(query.contains("LIKE ? ESCAPE '\\'"));
        assert!(query.contains("LIMIT 200"));
        assert!(!query.contains("(SELECT"));
        assert!(!query.contains("WHERE m.ZCHATSESSION = c.Z_PK"));
    }
}
