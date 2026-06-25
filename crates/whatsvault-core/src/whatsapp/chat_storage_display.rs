pub(super) fn first_nonempty<'a>(
    values: impl IntoIterator<Item = Option<&'a str>>,
) -> Option<String> {
    values
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn first_display_message_text<'a>(
    values: impl IntoIterator<Item = Option<&'a str>>,
) -> Option<String> {
    values.into_iter().flatten().find_map(display_message_text)
}

fn display_message_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    if looks_like_whatsapp_structured_system_event(value) {
        return Some("System event".to_owned());
    }

    let redacted = redact_internal_message_identifiers(value);
    let redacted = redacted.trim();
    if redacted.is_empty() {
        None
    } else {
        Some(redacted.to_owned())
    }
}

pub(super) fn display_chat_title(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return "Imported chat".to_owned();
    }

    let Some((local_part, domain)) = value.rsplit_once('@') else {
        return value.to_owned();
    };

    match domain {
        "g.us" => "Group chat".to_owned(),
        "lid" => "Participant".to_owned(),
        "s.whatsapp.net" => display_direct_jid_title(local_part),
        _ => value.to_owned(),
    }
}

fn display_direct_jid_title(local_part: &str) -> String {
    if local_part.len() >= 8 && local_part.bytes().all(|byte| byte.is_ascii_digit()) {
        format!("+{local_part}")
    } else if local_part.trim().is_empty() {
        "Imported chat".to_owned()
    } else {
        local_part.to_owned()
    }
}

fn looks_like_whatsapp_structured_system_event(value: &str) -> bool {
    value.starts_with('{')
        && value.ends_with('}')
        && value.contains("\"reason\"")
        && (value.contains("\"is_open_group\"")
            || value.contains("\"parent_group_jid\"")
            || value.contains("\"show_membership_string\""))
}

fn redact_internal_message_identifiers(value: &str) -> String {
    let mut redacted = String::with_capacity(value.len());
    let mut index = 0;

    while index < value.len() {
        let remaining = &value[index..];
        if let Some((consumed, replacement)) = internal_identifier_replacement(remaining) {
            redacted.push_str(replacement);
            index += consumed;
            continue;
        }

        let character = remaining
            .chars()
            .next()
            .expect("remaining slice is non-empty");
        redacted.push(character);
        index += character.len_utf8();
    }

    redacted
}

fn internal_identifier_replacement(value: &str) -> Option<(usize, &'static str)> {
    internal_mention_replacement(value).or_else(|| internal_jid_replacement(value))
}

fn internal_mention_replacement(value: &str) -> Option<(usize, &'static str)> {
    let remaining = value.strip_prefix('@')?;
    let digit_count = remaining.bytes().take_while(u8::is_ascii_digit).count();
    if digit_count >= 8 {
        Some((1 + digit_count, "@Participant"))
    } else {
        None
    }
}

fn internal_jid_replacement(value: &str) -> Option<(usize, &'static str)> {
    let digit_count = value.bytes().take_while(u8::is_ascii_digit).count();
    if digit_count < 8 {
        return None;
    }

    let suffix = &value[digit_count..];
    [
        ("@lid", "Participant"),
        ("@s.whatsapp.net", "Participant"),
        ("@g.us", "Group"),
    ]
    .into_iter()
    .find_map(|(domain, replacement)| {
        suffix
            .starts_with(domain)
            .then_some((digit_count + domain.len(), replacement))
    })
}

pub(super) fn first_readable_sender<'a>(
    values: impl IntoIterator<Item = Option<&'a str>>,
) -> Option<String> {
    values
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|value| is_readable_sender(value))
        .map(ToOwned::to_owned)
}

fn is_readable_sender(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }
    if value.contains('@') {
        return false;
    }
    if looks_like_phone_number(value) || looks_like_opaque_sender_identifier(value) {
        return false;
    }

    true
}

fn looks_like_phone_number(value: &str) -> bool {
    let digits = value
        .chars()
        .filter(|character| character.is_ascii_digit())
        .count();
    let separators = value
        .chars()
        .filter(|character| matches!(character, '+' | '-' | '(' | ')' | ' '))
        .count();

    digits >= 8 && digits + separators == value.chars().count()
}

fn looks_like_opaque_sender_identifier(value: &str) -> bool {
    let value = value.trim();
    value.len() >= 16
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '/' | '=')
        })
        && (value.contains('=') || value.contains('+') || value.contains('/'))
}

pub(super) fn filename_from_media_path(path: &str) -> String {
    path.replace('\\', "/")
        .rsplit('/')
        .next()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(path)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{display_chat_title, first_display_message_text, first_readable_sender};

    #[test]
    fn display_title_hides_backup_identifiers() {
        assert_eq!(display_chat_title("12345678901@g.us"), "Group chat");
        assert_eq!(display_chat_title("12345678901@lid"), "Participant");
        assert_eq!(
            display_chat_title("12345678901@s.whatsapp.net"),
            "+12345678901"
        );
    }

    #[test]
    fn message_text_redacts_internal_identifiers() {
        let text = first_display_message_text([Some(
            "Added @12345678901 and 12345678901@s.whatsapp.net to 12345678901@g.us",
        )]);

        assert_eq!(
            text.as_deref(),
            Some("Added @Participant and Participant to Group")
        );
    }

    #[test]
    fn readable_sender_skips_private_or_opaque_identifiers() {
        assert_eq!(
            first_readable_sender([
                Some("12345678901@s.whatsapp.net"),
                Some("abcedfghijklmnop=="),
                Some("Ana")
            ])
            .as_deref(),
            Some("Ana")
        );
    }
}
