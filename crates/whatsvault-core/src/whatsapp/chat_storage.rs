use std::{collections::HashSet, path::Path};

use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use thiserror::Error;

use crate::{
    media::attachment_kind_from_mime_or_filename, Attachment, Chat, ChatImport, ChatStorageSummary,
    Message, MessageTimestamp, SourceKind,
};

const MESSAGE_TABLE: &str = "ZWAMESSAGE";
const CHAT_TABLE: &str = "ZWACHATSESSION";
const MEDIA_ITEM_TABLE: &str = "ZWAMEDIAITEM";

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
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let Some(chat_schema) = table_schema(&connection, CHAT_TABLE)? else {
        return Ok(Vec::new());
    };
    if !chat_schema.has("Z_PK") {
        return Ok(Vec::new());
    }

    let message_schema = table_schema(&connection, MESSAGE_TABLE)?;
    let media_schema = table_schema(&connection, MEDIA_ITEM_TABLE)?;
    let title_expr = coalesced_text_expr(
        "c",
        &chat_schema,
        &["ZPARTNERNAME", "ZCONTACTJID"],
        "'Imported chat'",
    );
    let last_date_expr = chat_last_date_expr(&chat_schema, message_schema.as_ref());
    let latest_message_expr = latest_message_expr(message_schema.as_ref(), media_schema.as_ref());
    let message_count_expr = chat_message_count_expr(&chat_schema, message_schema.as_ref());
    let attachment_count_expr =
        chat_attachment_count_expr(message_schema.as_ref(), media_schema.as_ref());

    let query = format!(
        r#"
        SELECT
            CAST(c.Z_PK AS TEXT) AS id,
            {title_expr} AS title,
            {latest_message_expr} AS latest_message,
            {last_timestamp_expr} AS latest_message_timestamp,
            COALESCE({message_count_expr}, 0) AS message_count,
            COALESCE({attachment_count_expr}, 0) AS attachment_count,
            COALESCE({last_date_expr}, 0) AS sort_date
        FROM {CHAT_TABLE} c
        ORDER BY sort_date DESC, c.Z_PK DESC
        "#,
        last_timestamp_expr = cocoa_timestamp_expr(&last_date_expr),
    );
    let mut statement = connection.prepare(&query)?;
    let rows = statement.query_map([], |row| {
        let latest_timestamp = row
            .get::<_, Option<String>>(3)?
            .map(|raw| MessageTimestamp { raw });

        Ok(Chat {
            id: row.get(0)?,
            title: row.get::<_, String>(1)?,
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

    Ok(chats)
}

pub fn import_chat_storage_chat<P>(path: P, chat_id: &str) -> Result<ChatImport>
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

    let media_schema = table_schema(&connection, MEDIA_ITEM_TABLE)?;
    let media_join = media_join_expr(&message_schema, media_schema.as_ref());
    let message_text_expr = nullable_text_expr("m", &message_schema, "ZTEXT");
    let from_jid_expr = nullable_text_expr("m", &message_schema, "ZFROMJID");
    let push_name_expr = nullable_text_expr("m", &message_schema, "ZPUSHNAME");
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
            {from_jid_expr} AS from_jid,
            {push_name_expr} AS push_name,
            {media_pk_expr} AS media_pk,
            {media_path_expr} AS media_path,
            {media_type_expr} AS media_type,
            {media_title_expr} AS media_title,
            {media_size_expr} AS media_size
        FROM {MESSAGE_TABLE} m
        {media_join}
        WHERE m.ZCHATSESSION = ?1
        ORDER BY {order_expr}, m.Z_PK
        "#,
        message_timestamp_expr = cocoa_timestamp_expr(&message_date_expr),
    );
    let mut statement = connection.prepare(&query)?;
    let rows = statement.query_map(params![chat_id], |row| {
        Ok(ChatStorageMessageRow {
            message_pk: row.get(0)?,
            timestamp: row.get(1)?,
            is_from_me: row.get(2)?,
            message_text: row.get(3)?,
            from_jid: row.get(4)?,
            push_name: row.get(5)?,
            media_pk: row.get(6)?,
            media_path: row.get(7)?,
            media_type: row.get(8)?,
            media_title: row.get(9)?,
            media_size: row.get(10)?,
        })
    })?;

    let mut messages = Vec::new();
    let mut attachments = Vec::new();
    let mut seen_attachment_ids = HashSet::new();

    for row in rows {
        let row = row?;
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

    Ok(ChatImport {
        source_kind: SourceKind::IphoneBackup,
        transcript_name: Some(title),
        messages,
        attachments,
        issues: Vec::new(),
    })
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

        first_nonempty([self.push_name.as_deref(), self.from_jid.as_deref()])
    }

    fn body(&self) -> String {
        let media_filename = self.media_path.as_deref().map(filename_from_media_path);

        first_nonempty([
            self.message_text.as_deref(),
            self.media_title.as_deref(),
            media_filename.as_deref(),
        ])
        .unwrap_or_default()
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

    Ok(connection
        .query_row(&query, params![chat_id], |row| row.get::<_, String>(0))
        .optional()?
        .unwrap_or_else(|| format!("Chat {chat_id}")))
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

fn chat_last_date_expr(chat_schema: &TableSchema, message_schema: Option<&TableSchema>) -> String {
    if chat_schema.has("ZLASTMESSAGEDATE") {
        return "c.ZLASTMESSAGEDATE".to_owned();
    }

    if message_schema.is_some_and(|schema| schema.has("ZCHATSESSION") && schema.has("ZMESSAGEDATE"))
    {
        return format!(
            "(SELECT MAX(m.ZMESSAGEDATE) FROM {MESSAGE_TABLE} m WHERE m.ZCHATSESSION = c.Z_PK)"
        );
    }

    "NULL".to_owned()
}

fn latest_message_expr(
    message_schema: Option<&TableSchema>,
    media_schema: Option<&TableSchema>,
) -> String {
    let Some(message_schema) = message_schema else {
        return "NULL".to_owned();
    };
    if !message_schema.has("ZCHATSESSION") || !message_schema.has("Z_PK") {
        return "NULL".to_owned();
    }

    let message_text_expr = nullable_text_expr("m", message_schema, "ZTEXT");
    let media_join = media_join_expr(message_schema, media_schema);
    let media_title_expr = media_text_column_expr(media_schema, "ZTITLE");
    let media_path_expr = media_text_column_expr(media_schema, "ZMEDIALOCALPATH");
    let order_expr = message_order_expr(message_schema);

    format!(
        r#"
        (
            SELECT COALESCE(
                {message_text_expr},
                {media_title_expr},
                {media_path_expr}
            )
            FROM {MESSAGE_TABLE} m
            {media_join}
            WHERE m.ZCHATSESSION = c.Z_PK
            ORDER BY {order_expr} DESC, m.Z_PK DESC
            LIMIT 1
        )
        "#
    )
}

fn chat_message_count_expr(
    chat_schema: &TableSchema,
    message_schema: Option<&TableSchema>,
) -> String {
    if chat_schema.has("ZMESSAGECOUNTER") {
        return "c.ZMESSAGECOUNTER".to_owned();
    }

    if message_schema.is_some_and(|schema| schema.has("ZCHATSESSION")) {
        return format!("(SELECT COUNT(*) FROM {MESSAGE_TABLE} m WHERE m.ZCHATSESSION = c.Z_PK)");
    }

    "0".to_owned()
}

fn chat_attachment_count_expr(
    message_schema: Option<&TableSchema>,
    media_schema: Option<&TableSchema>,
) -> String {
    let Some(message_schema) = message_schema else {
        return "0".to_owned();
    };
    let Some(media_schema) = media_schema else {
        return "0".to_owned();
    };
    if !message_schema.has("Z_PK") || !message_schema.has("ZCHATSESSION") {
        return "0".to_owned();
    }

    if media_schema.has("ZMESSAGE") {
        return format!(
            r#"
            (
                SELECT COUNT(*)
                FROM {MEDIA_ITEM_TABLE} mi
                JOIN {MESSAGE_TABLE} m ON mi.ZMESSAGE = m.Z_PK
                WHERE m.ZCHATSESSION = c.Z_PK
            )
            "#
        );
    }

    if message_schema.has("ZMEDIAITEM") && media_schema.has("Z_PK") {
        return format!(
            r#"
            (
                SELECT COUNT(*)
                FROM {MESSAGE_TABLE} m
                JOIN {MEDIA_ITEM_TABLE} mi ON m.ZMEDIAITEM = mi.Z_PK
                WHERE m.ZCHATSESSION = c.Z_PK
            )
            "#
        );
    }

    "0".to_owned()
}

fn media_join_expr(message_schema: &TableSchema, media_schema: Option<&TableSchema>) -> String {
    let Some(media_schema) = media_schema else {
        return String::new();
    };

    if message_schema.has("ZMEDIAITEM") && media_schema.has("Z_PK") {
        return format!(
            "LEFT JOIN {} mi ON m.ZMEDIAITEM = mi.Z_PK",
            media_schema.name
        );
    }

    if message_schema.has("Z_PK") && media_schema.has("ZMESSAGE") {
        return format!("LEFT JOIN {} mi ON mi.ZMESSAGE = m.Z_PK", media_schema.name);
    }

    String::new()
}

fn message_order_expr(message_schema: &TableSchema) -> String {
    if message_schema.has("ZSORT") {
        return "m.ZSORT".to_owned();
    }
    if message_schema.has("ZMESSAGEDATE") {
        return "m.ZMESSAGEDATE".to_owned();
    }

    "m.Z_PK".to_owned()
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

fn first_nonempty<'a>(values: impl IntoIterator<Item = Option<&'a str>>) -> Option<String> {
    values
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn filename_from_media_path(path: &str) -> String {
    path.replace('\\', "/")
        .rsplit('/')
        .next()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(path)
        .to_owned()
}

fn nonnegative_i64_to_u64(value: i64) -> u64 {
    u64::try_from(value).unwrap_or_default()
}
