use std::path::Path;

use rusqlite::Connection;
use tempfile::tempdir;
use whatsvault_core::{
    whatsapp::chat_storage::{
        import_chat_storage_chat, list_chat_storage_chats, summarize_chat_storage,
    },
    AttachmentKind, SourceKind,
};

fn create_chat_storage(path: &Path) {
    let connection = Connection::open(path).unwrap();
    connection
        .execute_batch(
            r#"
            CREATE TABLE ZWAMESSAGE (
                Z_PK INTEGER PRIMARY KEY,
                ZTEXT TEXT
            );
            CREATE TABLE ZWACHATSESSION (
                Z_PK INTEGER PRIMARY KEY,
                ZCONTACTJID TEXT
            );
            CREATE TABLE ZWAMEDIAITEM (
                Z_PK INTEGER PRIMARY KEY,
                ZMESSAGE INTEGER
            );

            INSERT INTO ZWAMESSAGE (ZTEXT) VALUES ('hello'), ('reply'), ('photo');
            INSERT INTO ZWACHATSESSION (ZCONTACTJID) VALUES ('one@s.whatsapp.net'), ('group@g.us');
            INSERT INTO ZWAMEDIAITEM (ZMESSAGE) VALUES (3);
            "#,
        )
        .unwrap();
}

fn create_threaded_chat_storage(path: &Path) {
    let connection = Connection::open(path).unwrap();
    connection
        .execute_batch(
            r#"
            CREATE TABLE ZWACHATSESSION (
                Z_PK INTEGER PRIMARY KEY,
                ZCONTACTJID TEXT,
                ZPARTNERNAME TEXT,
                ZMESSAGECOUNTER INTEGER,
                ZLASTMESSAGEDATE REAL,
                ZLASTMESSAGE INTEGER
            );
            CREATE TABLE ZWAMESSAGE (
                Z_PK INTEGER PRIMARY KEY,
                ZCHATSESSION INTEGER,
                ZSORT INTEGER,
                ZISFROMME INTEGER,
                ZMESSAGETYPE INTEGER,
                ZMESSAGEDATE REAL,
                ZTEXT TEXT,
                ZFROMJID TEXT,
                ZPUSHNAME TEXT,
                ZMEDIAITEM INTEGER
            );
            CREATE TABLE ZWAMEDIAITEM (
                Z_PK INTEGER PRIMARY KEY,
                ZMESSAGE INTEGER,
                ZMEDIALOCALPATH TEXT,
                ZVCARDSTRING TEXT,
                ZTITLE TEXT,
                ZFILESIZE INTEGER
            );

            INSERT INTO ZWACHATSESSION
                (Z_PK, ZCONTACTJID, ZPARTNERNAME, ZMESSAGECOUNTER, ZLASTMESSAGEDATE, ZLASTMESSAGE)
            VALUES
                (1, 'design-preview@s.whatsapp.net', 'Design Preview', 3, 120, 3),
                (2, 'archive@g.us', NULL, 1, 60, 4);

            INSERT INTO ZWAMESSAGE
                (Z_PK, ZCHATSESSION, ZSORT, ZISFROMME, ZMESSAGETYPE, ZMESSAGEDATE, ZTEXT, ZFROMJID, ZPUSHNAME, ZMEDIAITEM)
            VALUES
                (1, 1, 1, 0, 0, 60, 'hello from backup', 'contact-one@s.whatsapp.net', 'Contact One', NULL),
                (2, 1, 2, 1, 1, 90, 'photo attached', NULL, NULL, 10),
                (3, 1, 3, 0, 3, 120, '', 'alex@s.whatsapp.net', 'Alex', 11),
                (4, 2, 1, 0, 0, 60, 'archived message', 'archive@g.us', 'Archive', NULL);

            INSERT INTO ZWAMEDIAITEM
                (Z_PK, ZMESSAGE, ZMEDIALOCALPATH, ZVCARDSTRING, ZTITLE, ZFILESIZE)
            VALUES
                (10, 2, 'Message/Media/photo.jpg', 'image/jpeg', 'photo.jpg', 1234),
                (11, 3, 'Message/Media/voice.opus', 'audio/ogg', 'voice.opus', 4321);
            "#,
        )
        .unwrap();
}

fn create_minimal_chat_storage(path: &Path) {
    let connection = Connection::open(path).unwrap();
    connection
        .execute_batch(
            r#"
            CREATE TABLE ZWACHATSESSION (
                Z_PK INTEGER PRIMARY KEY,
                ZCONTACTJID TEXT
            );
            CREATE TABLE ZWAMESSAGE (
                Z_PK INTEGER PRIMARY KEY,
                ZCHATSESSION INTEGER,
                ZTEXT TEXT
            );

            INSERT INTO ZWACHATSESSION (Z_PK, ZCONTACTJID)
            VALUES (7, 'minimal@s.whatsapp.net');
            INSERT INTO ZWAMESSAGE (Z_PK, ZCHATSESSION, ZTEXT)
            VALUES (70, 7, 'minimal schema message');
            "#,
        )
        .unwrap();
}

#[test]
fn summarizes_known_ios_whatsapp_chat_storage_tables() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    create_chat_storage(&chat_storage);

    let summary = summarize_chat_storage(&chat_storage).unwrap();

    assert_eq!(summary.message_count, Some(3));
    assert_eq!(summary.chat_count, Some(2));
    assert_eq!(summary.media_item_count, Some(1));
}

#[test]
fn leaves_missing_table_counts_empty_without_failing() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    let connection = Connection::open(&chat_storage).unwrap();
    connection
        .execute_batch(
            r#"
            CREATE TABLE ZWAMESSAGE (
                Z_PK INTEGER PRIMARY KEY,
                ZTEXT TEXT
            );
            INSERT INTO ZWAMESSAGE (ZTEXT) VALUES ('hello');
            "#,
        )
        .unwrap();

    let summary = summarize_chat_storage(&chat_storage).unwrap();

    assert_eq!(summary.message_count, Some(1));
    assert_eq!(summary.chat_count, None);
    assert_eq!(summary.media_item_count, None);
}

#[test]
fn lists_chat_storage_chats_sorted_by_latest_message() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    create_threaded_chat_storage(&chat_storage);

    let chats = list_chat_storage_chats(&chat_storage).unwrap();

    assert_eq!(chats.len(), 2);
    assert_eq!(chats[0].id, "1");
    assert_eq!(chats[0].title, "Design Preview");
    assert_eq!(chats[0].latest_message.as_deref(), Some("voice.opus"));
    assert_eq!(
        chats[0]
            .latest_message_timestamp
            .as_ref()
            .map(|time| time.raw.as_str()),
        Some("01/01/2001, 00:02")
    );
    assert_eq!(chats[0].message_count, 3);
    assert_eq!(chats[0].attachment_count, 2);
    assert_eq!(chats[1].id, "2");
    assert_eq!(chats[1].title, "archive@g.us");
}

#[test]
fn imports_selected_chat_storage_thread_as_normalized_chat_import() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    create_threaded_chat_storage(&chat_storage);

    let imported = import_chat_storage_chat(&chat_storage, "1").unwrap();

    assert_eq!(imported.source_kind, SourceKind::IphoneBackup);
    assert_eq!(imported.transcript_name.as_deref(), Some("Design Preview"));
    assert!(imported.issues.is_empty());
    assert_eq!(imported.messages.len(), 3);
    assert_eq!(imported.attachments.len(), 2);

    assert_eq!(imported.messages[0].sender.as_deref(), Some("Contact One"));
    assert_eq!(imported.messages[0].body, "hello from backup");
    assert_eq!(imported.messages[0].timestamp.raw, "01/01/2001, 00:01");

    assert_eq!(imported.messages[1].sender.as_deref(), Some("You"));
    assert_eq!(imported.messages[1].body, "photo attached");
    assert_eq!(
        imported.messages[1].attachment_ids,
        vec!["chat-storage-media-00000010"]
    );

    assert_eq!(imported.messages[2].sender.as_deref(), Some("Alex"));
    assert_eq!(imported.messages[2].body, "voice.opus");
    assert_eq!(
        imported.messages[2].attachment_ids,
        vec!["chat-storage-media-00000011"]
    );

    assert_eq!(imported.attachments[0].id, "chat-storage-media-00000010");
    assert_eq!(
        imported.attachments[0].archive_path,
        "Message/Media/photo.jpg"
    );
    assert_eq!(imported.attachments[0].filename, "photo.jpg");
    assert_eq!(imported.attachments[0].kind, AttachmentKind::Photo);
    assert_eq!(imported.attachments[0].size_bytes, 1234);

    assert_eq!(imported.attachments[1].id, "chat-storage-media-00000011");
    assert_eq!(imported.attachments[1].kind, AttachmentKind::Audio);
}

#[test]
fn imports_minimal_chat_storage_schema_without_optional_columns() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    create_minimal_chat_storage(&chat_storage);

    let chats = list_chat_storage_chats(&chat_storage).unwrap();
    let imported = import_chat_storage_chat(&chat_storage, "7").unwrap();

    assert_eq!(chats.len(), 1);
    assert_eq!(chats[0].id, "7");
    assert_eq!(chats[0].title, "minimal@s.whatsapp.net");
    assert_eq!(chats[0].message_count, 1);
    assert_eq!(chats[0].attachment_count, 0);
    assert_eq!(
        imported.transcript_name.as_deref(),
        Some("minimal@s.whatsapp.net")
    );
    assert_eq!(imported.messages.len(), 1);
    assert_eq!(imported.messages[0].body, "minimal schema message");
    assert_eq!(imported.messages[0].timestamp.raw, "Unknown time");
    assert!(imported.attachments.is_empty());
}
