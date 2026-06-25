use std::path::Path;

use rusqlite::Connection;
use tempfile::tempdir;
use whatsvault_core::{
    whatsapp::chat_storage::{
        import_chat_storage_chat, import_chat_storage_chat_recent, list_chat_storage_chats,
        list_chat_storage_chats_limited, search_chat_storage_chat_recent,
        search_chat_storage_chats_limited, summarize_chat_storage,
    },
    AttachmentKind, ImportIssueCode, SourceKind,
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

fn create_stale_counter_chat_storage(path: &Path) {
    let connection = Connection::open(path).unwrap();
    connection
        .execute_batch(
            r#"
            CREATE TABLE ZWACHATSESSION (
                Z_PK INTEGER PRIMARY KEY,
                ZCONTACTJID TEXT,
                ZMESSAGECOUNTER INTEGER,
                ZLASTMESSAGEDATE REAL
            );
            CREATE TABLE ZWAMESSAGE (
                Z_PK INTEGER PRIMARY KEY,
                ZCHATSESSION INTEGER,
                ZMESSAGEDATE REAL,
                ZTEXT TEXT
            );

            INSERT INTO ZWACHATSESSION (Z_PK, ZCONTACTJID, ZMESSAGECOUNTER, ZLASTMESSAGEDATE)
            VALUES
                (1, 'stale@s.whatsapp.net', 105, 120),
                (2, 'importable@s.whatsapp.net', 1, 60);

            INSERT INTO ZWAMESSAGE (Z_PK, ZCHATSESSION, ZMESSAGEDATE, ZTEXT)
            VALUES (20, 2, 60, 'importable message');
            "#,
        )
        .unwrap();
}

fn create_unresolved_media_chat_storage(path: &Path) {
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
                ZMEDIAITEM INTEGER,
                ZTEXT TEXT
            );
            CREATE TABLE ZWAMEDIAITEM (
                Z_PK INTEGER PRIMARY KEY,
                ZMESSAGE INTEGER,
                ZMEDIALOCALPATH TEXT,
                ZTITLE TEXT
            );

            INSERT INTO ZWACHATSESSION (Z_PK, ZCONTACTJID)
            VALUES (1, 'media@s.whatsapp.net');
            INSERT INTO ZWAMESSAGE (Z_PK, ZCHATSESSION, ZMEDIAITEM, ZTEXT)
            VALUES
                (10, 1, 100, 'usable media'),
                (11, 1, 101, 'unresolved media');
            INSERT INTO ZWAMEDIAITEM (Z_PK, ZMESSAGE, ZMEDIALOCALPATH, ZTITLE)
            VALUES
                (100, 10, 'Message/Media/photo.jpg', NULL),
                (101, 11, NULL, '');
            "#,
        )
        .unwrap();
}

fn create_sender_resolution_chat_storage(path: &Path) {
    let connection = Connection::open(path).unwrap();
    connection
        .execute_batch(
            r#"
            CREATE TABLE ZWACHATSESSION (
                Z_PK INTEGER PRIMARY KEY,
                ZCONTACTJID TEXT,
                ZPARTNERNAME TEXT
            );
            CREATE TABLE ZWAMESSAGE (
                Z_PK INTEGER PRIMARY KEY,
                ZCHATSESSION INTEGER,
                ZSORT INTEGER,
                ZISFROMME INTEGER,
                ZGROUPMEMBER INTEGER,
                ZMESSAGEDATE REAL,
                ZTEXT TEXT,
                ZFROMJID TEXT,
                ZPUSHNAME TEXT
            );
            CREATE TABLE ZWAGROUPMEMBER (
                Z_PK INTEGER PRIMARY KEY,
                ZCONTACTNAME TEXT,
                ZFIRSTNAME TEXT,
                ZMEMBERJID TEXT
            );
            CREATE TABLE ZWAPROFILEPUSHNAME (
                Z_PK INTEGER PRIMARY KEY,
                ZJID TEXT,
                ZPUSHNAME TEXT
            );

            INSERT INTO ZWACHATSESSION (Z_PK, ZCONTACTJID, ZPARTNERNAME)
            VALUES (1, 'group@g.us', 'Group Chat');
            INSERT INTO ZWAGROUPMEMBER (Z_PK, ZCONTACTNAME, ZFIRSTNAME, ZMEMBERJID)
            VALUES
                (10, 'Readable Member', NULL, 'member-one@s.whatsapp.net'),
                (11, NULL, NULL, 'member-two@s.whatsapp.net');
            INSERT INTO ZWAPROFILEPUSHNAME (Z_PK, ZJID, ZPUSHNAME)
            VALUES (20, 'member-two@s.whatsapp.net', 'Profile Name');
            INSERT INTO ZWAMESSAGE
                (Z_PK, ZCHATSESSION, ZSORT, ZISFROMME, ZGROUPMEMBER, ZMESSAGEDATE, ZTEXT, ZFROMJID, ZPUSHNAME)
            VALUES
                (1, 1, 1, 0, 10, 60, 'member message', 'member-one@s.whatsapp.net', 'CIfh6c4GIABIAZABAPABAg=='),
                (2, 1, 2, 0, 11, 90, 'profile message', 'member-two@s.whatsapp.net', 'CIgh6c4GIABIAZABAPABAg=='),
                (3, 1, 3, 0, NULL, 120, 'unknown message', 'CJhh6c4GIABIAZABAPABAg==', 'CJhh6c4GIABIAZABAPABAg==');
            "#,
        )
        .unwrap();
}

fn create_internal_text_chat_storage(path: &Path) {
    let connection = Connection::open(path).unwrap();
    connection
        .execute_batch(
            r#"
            CREATE TABLE ZWACHATSESSION (
                Z_PK INTEGER PRIMARY KEY,
                ZCONTACTJID TEXT,
                ZPARTNERNAME TEXT
            );
            CREATE TABLE ZWAMESSAGE (
                Z_PK INTEGER PRIMARY KEY,
                ZCHATSESSION INTEGER,
                ZSORT INTEGER,
                ZMESSAGEDATE REAL,
                ZTEXT TEXT
            );

            INSERT INTO ZWACHATSESSION (Z_PK, ZCONTACTJID, ZPARTNERNAME)
            VALUES (1, 'group@g.us', 'Group Chat');
            INSERT INTO ZWAMESSAGE (Z_PK, ZCHATSESSION, ZSORT, ZMESSAGEDATE, ZTEXT)
            VALUES
                (1, 1, 1, 60, '@155331871162418 Please check this'),
                (2, 1, 2, 90, '212970969813071@lid'),
                (3, 1, 3, 120, '{"reason":1,"is_open_group":false,"parent_group_jid":"123@g.us","show_membership_string":false}');
            "#,
        )
        .unwrap();
}

fn create_raw_jid_title_chat_storage(path: &Path) {
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
                ZMESSAGEDATE REAL,
                ZTEXT TEXT
            );

            INSERT INTO ZWACHATSESSION (Z_PK, ZCONTACTJID)
            VALUES
                (1, '120363406139150653@g.us'),
                (2, '15551230000@s.whatsapp.net'),
                (3, '212970969813071@lid');
            INSERT INTO ZWAMESSAGE (Z_PK, ZCHATSESSION, ZMESSAGEDATE, ZTEXT)
            VALUES
                (10, 1, 30, 'group message'),
                (20, 2, 20, 'direct message'),
                (30, 3, 10, 'lid message');
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
    assert_eq!(chats[0].latest_message.as_deref(), Some("Voice message"));
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
    assert_eq!(chats[1].title, "Group chat");
}

#[test]
fn lists_media_latest_message_as_a_label_instead_of_a_backup_path() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    let connection = Connection::open(&chat_storage).unwrap();
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
                ZSORT INTEGER,
                ZMEDIAITEM INTEGER,
                ZTEXT TEXT
            );
            CREATE TABLE ZWAMEDIAITEM (
                Z_PK INTEGER PRIMARY KEY,
                ZMESSAGE INTEGER,
                ZMEDIALOCALPATH TEXT,
                ZVCARDSTRING TEXT,
                ZTITLE TEXT
            );

            INSERT INTO ZWACHATSESSION (Z_PK, ZCONTACTJID)
            VALUES (1, 'media@s.whatsapp.net');
            INSERT INTO ZWAMESSAGE (Z_PK, ZCHATSESSION, ZSORT, ZMEDIAITEM, ZTEXT)
            VALUES
                (10, 1, 1, NULL, 'text before media'),
                (11, 1, 2, 100, '');
            INSERT INTO ZWAMEDIAITEM (Z_PK, ZMESSAGE, ZMEDIALOCALPATH, ZVCARDSTRING, ZTITLE)
            VALUES (100, 11, 'Message/Media/photo.jpg', 'image/jpeg', 'photo.jpg');
            "#,
        )
        .unwrap();

    let chats = list_chat_storage_chats(&chat_storage).unwrap();

    assert_eq!(chats[0].latest_message.as_deref(), Some("Photo"));
}

#[test]
fn lists_chat_storage_chats_with_an_explicit_limit() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    create_threaded_chat_storage(&chat_storage);

    let chats = list_chat_storage_chats_limited(&chat_storage, 1).unwrap();

    assert_eq!(chats.len(), 1);
    assert_eq!(chats[0].id, "1");
}

#[test]
fn searches_chat_storage_chats_outside_the_initial_recent_window() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    let connection = Connection::open(&chat_storage).unwrap();
    connection
        .execute_batch(
            r#"
            CREATE TABLE ZWACHATSESSION (
                Z_PK INTEGER PRIMARY KEY,
                ZCONTACTJID TEXT,
                ZPARTNERNAME TEXT
            );
            CREATE TABLE ZWAMESSAGE (
                Z_PK INTEGER PRIMARY KEY,
                ZCHATSESSION INTEGER,
                ZMESSAGEDATE REAL,
                ZTEXT TEXT
            );

            INSERT INTO ZWACHATSESSION (Z_PK, ZCONTACTJID, ZPARTNERNAME)
            VALUES
                (1, 'recent@s.whatsapp.net', 'Recent Chat'),
                (2, 'needle@s.whatsapp.net', 'Needle Archive');
            INSERT INTO ZWAMESSAGE (Z_PK, ZCHATSESSION, ZMESSAGEDATE, ZTEXT)
            VALUES
                (1, 1, 900, 'newest visible chat'),
                (2, 2, 100, 'older hidden chat');
            "#,
        )
        .unwrap();

    let recent = list_chat_storage_chats_limited(&chat_storage, 1).unwrap();
    let searched = search_chat_storage_chats_limited(&chat_storage, "needle", 10).unwrap();

    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].title, "Recent Chat");
    assert_eq!(searched.len(), 1);
    assert_eq!(searched[0].title, "Needle Archive");
    assert_eq!(
        searched[0].latest_message.as_deref(),
        Some("older hidden chat")
    );
}

#[test]
fn chat_storage_chat_search_treats_like_wildcards_as_literal_text() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    let connection = Connection::open(&chat_storage).unwrap();
    connection
        .execute_batch(
            r#"
            CREATE TABLE ZWACHATSESSION (
                Z_PK INTEGER PRIMARY KEY,
                ZCONTACTJID TEXT,
                ZPARTNERNAME TEXT
            );
            INSERT INTO ZWACHATSESSION (Z_PK, ZCONTACTJID, ZPARTNERNAME)
            VALUES
                (1, 'percent@s.whatsapp.net', '100% Real Chat'),
                (2, 'letter@s.whatsapp.net', '100x Real Chat');
            "#,
        )
        .unwrap();

    let chats = search_chat_storage_chats_limited(&chat_storage, "100%", 10).unwrap();

    assert_eq!(chats.len(), 1);
    assert_eq!(chats[0].title, "100% Real Chat");
}

#[test]
fn lists_actual_importable_message_count_when_chat_counter_is_stale() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    create_stale_counter_chat_storage(&chat_storage);

    let chats = list_chat_storage_chats(&chat_storage).unwrap();
    let stale_chat = chats.iter().find(|chat| chat.id == "1").unwrap();
    let imported = import_chat_storage_chat(&chat_storage, "1").unwrap();

    assert_eq!(chats.len(), 2);
    assert_eq!(stale_chat.message_count, imported.messages.len() as u64);
    assert_eq!(stale_chat.message_count, 0);
}

#[test]
fn sorts_importable_chats_before_stale_chat_metadata() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    create_stale_counter_chat_storage(&chat_storage);

    let chats = list_chat_storage_chats(&chat_storage).unwrap();

    assert_eq!(chats.len(), 2);
    assert_eq!(chats[0].id, "2");
    assert_eq!(
        chats[0].latest_message.as_deref(),
        Some("importable message")
    );
    assert_eq!(chats[0].message_count, 1);
}

#[test]
fn lists_only_importable_attachment_count_for_chat_storage_threads() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    create_unresolved_media_chat_storage(&chat_storage);

    let chats = list_chat_storage_chats(&chat_storage).unwrap();
    let imported = import_chat_storage_chat(&chat_storage, "1").unwrap();

    assert_eq!(chats.len(), 1);
    assert_eq!(chats[0].attachment_count, imported.attachments.len() as u64);
    assert_eq!(chats[0].attachment_count, 1);
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
    assert_eq!(imported.messages[2].body, "Voice message");
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
fn resolves_backup_senders_without_exposing_opaque_identifiers() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    create_sender_resolution_chat_storage(&chat_storage);

    let imported = import_chat_storage_chat(&chat_storage, "1").unwrap();

    let senders: Vec<Option<&str>> = imported
        .messages
        .iter()
        .map(|message| message.sender.as_deref())
        .collect();
    assert_eq!(
        senders,
        vec![
            Some("Readable Member"),
            Some("Profile Name"),
            Some("Participant")
        ]
    );
}

#[test]
fn normalizes_internal_backup_message_text_for_display() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    create_internal_text_chat_storage(&chat_storage);

    let chats = list_chat_storage_chats(&chat_storage).unwrap();
    let imported = import_chat_storage_chat(&chat_storage, "1").unwrap();

    assert_eq!(chats[0].latest_message.as_deref(), Some("System event"));
    assert_eq!(imported.messages[0].body, "@Participant Please check this");
    assert_eq!(imported.messages[1].body, "Participant");
    assert_eq!(imported.messages[2].body, "System event");
}

#[test]
fn normalizes_raw_whatsapp_jid_chat_titles_for_display() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    create_raw_jid_title_chat_storage(&chat_storage);

    let chats = list_chat_storage_chats(&chat_storage).unwrap();
    let imported_group = import_chat_storage_chat(&chat_storage, "1").unwrap();

    assert_eq!(chats[0].title, "Group chat");
    assert_eq!(chats[1].title, "+15551230000");
    assert_eq!(chats[2].title, "Participant");
    assert_eq!(
        imported_group.transcript_name.as_deref(),
        Some("Group chat")
    );
}

#[test]
fn imports_recent_chat_storage_messages_in_chronological_order() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    create_threaded_chat_storage(&chat_storage);

    let imported = import_chat_storage_chat_recent(&chat_storage, "1", 2).unwrap();

    assert_eq!(imported.messages.len(), 2);
    assert_eq!(imported.messages[0].body, "photo attached");
    assert_eq!(imported.messages[1].body, "Voice message");
    assert_eq!(
        imported.messages[0].attachment_ids,
        vec!["chat-storage-media-00000010"]
    );
    assert_eq!(
        imported.messages[1].attachment_ids,
        vec!["chat-storage-media-00000011"]
    );
    assert_eq!(imported.attachments.len(), 2);
}

#[test]
fn searches_selected_chat_storage_thread_with_bounded_latest_matches() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    let connection = Connection::open(&chat_storage).unwrap();
    connection
        .execute_batch(
            r#"
            CREATE TABLE ZWACHATSESSION (
                Z_PK INTEGER PRIMARY KEY,
                ZCONTACTJID TEXT,
                ZPARTNERNAME TEXT
            );
            CREATE TABLE ZWAMESSAGE (
                Z_PK INTEGER PRIMARY KEY,
                ZCHATSESSION INTEGER,
                ZSORT INTEGER,
                ZISFROMME INTEGER,
                ZMESSAGEDATE REAL,
                ZTEXT TEXT,
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

            INSERT INTO ZWACHATSESSION (Z_PK, ZCONTACTJID, ZPARTNERNAME)
            VALUES (1, 'search@s.whatsapp.net', 'Search Chat');
            INSERT INTO ZWAMESSAGE
                (Z_PK, ZCHATSESSION, ZSORT, ZISFROMME, ZMESSAGEDATE, ZTEXT, ZPUSHNAME, ZMEDIAITEM)
            VALUES
                (1, 1, 1, 0, 60, 'needle older match', 'Ana', NULL),
                (2, 1, 2, 1, 90, 'ordinary message', NULL, NULL),
                (3, 1, 3, 0, 120, 'needle middle match', 'Bruno', 30),
                (4, 1, 4, 0, 150, 'needle latest match', 'Carla', NULL);
            INSERT INTO ZWAMEDIAITEM
                (Z_PK, ZMESSAGE, ZMEDIALOCALPATH, ZVCARDSTRING, ZTITLE, ZFILESIZE)
            VALUES (30, 3, 'Message/Media/photo.jpg', 'image/jpeg', 'photo.jpg', 1234);
            "#,
        )
        .unwrap();

    let imported = search_chat_storage_chat_recent(&chat_storage, "1", "needle", 2).unwrap();

    assert_eq!(imported.transcript_name.as_deref(), Some("Search Chat"));
    assert_eq!(
        imported
            .messages
            .iter()
            .map(|message| message.body.as_str())
            .collect::<Vec<_>>(),
        vec!["needle middle match", "needle latest match"]
    );
    assert_eq!(imported.messages[0].sender.as_deref(), Some("Bruno"));
    assert_eq!(
        imported.messages[0].attachment_ids,
        vec!["chat-storage-media-00000030"]
    );
    assert_eq!(imported.attachments.len(), 1);
    assert_eq!(imported.attachments[0].kind, AttachmentKind::Photo);
    assert_eq!(imported.issues.len(), 1);
    assert_eq!(
        imported.issues[0].code,
        ImportIssueCode::SearchResultsTruncated
    );
}

#[test]
fn selected_chat_storage_search_can_match_sender_names() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    create_threaded_chat_storage(&chat_storage);

    let imported = search_chat_storage_chat_recent(&chat_storage, "1", "alex", 20).unwrap();

    assert_eq!(imported.messages.len(), 1);
    assert_eq!(imported.messages[0].sender.as_deref(), Some("Alex"));
    assert_eq!(imported.messages[0].body, "Voice message");
    assert!(imported.issues.is_empty());
}

#[test]
fn selected_chat_storage_search_treats_like_wildcards_as_literal_text() {
    let root = tempdir().unwrap();
    let chat_storage = root.path().join("ChatStorage.sqlite");
    let connection = Connection::open(&chat_storage).unwrap();
    connection
        .execute_batch(
            r#"
            CREATE TABLE ZWACHATSESSION (
                Z_PK INTEGER PRIMARY KEY,
                ZCONTACTJID TEXT,
                ZPARTNERNAME TEXT
            );
            CREATE TABLE ZWAMESSAGE (
                Z_PK INTEGER PRIMARY KEY,
                ZCHATSESSION INTEGER,
                ZSORT INTEGER,
                ZISFROMME INTEGER,
                ZMESSAGEDATE REAL,
                ZTEXT TEXT
            );

            INSERT INTO ZWACHATSESSION (Z_PK, ZCONTACTJID, ZPARTNERNAME)
            VALUES (1, 'wildcards@s.whatsapp.net', 'Wildcard Chat');
            INSERT INTO ZWAMESSAGE
                (Z_PK, ZCHATSESSION, ZSORT, ZISFROMME, ZMESSAGEDATE, ZTEXT)
            VALUES
                (1, 1, 1, 0, 60, 'discount 100% real'),
                (2, 1, 2, 0, 90, 'discount 100x fake');
            "#,
        )
        .unwrap();

    let imported = search_chat_storage_chat_recent(&chat_storage, "1", "100%", 20).unwrap();

    assert_eq!(
        imported
            .messages
            .iter()
            .map(|message| message.body.as_str())
            .collect::<Vec<_>>(),
        vec!["discount 100% real"]
    );
    assert!(imported.issues.is_empty());
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
    assert_eq!(chats[0].title, "minimal");
    assert_eq!(chats[0].message_count, 1);
    assert_eq!(chats[0].attachment_count, 0);
    assert_eq!(imported.transcript_name.as_deref(), Some("minimal"));
    assert_eq!(imported.messages.len(), 1);
    assert_eq!(imported.messages[0].body, "minimal schema message");
    assert_eq!(imported.messages[0].timestamp.raw, "Unknown time");
    assert!(imported.attachments.is_empty());
}
