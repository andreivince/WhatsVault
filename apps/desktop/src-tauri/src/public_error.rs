#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PublicError {
    SelectedSourceUnreadable,
    SelectedSourceImportFailed,
    AttachmentPreviewFailed,
    BackupChatListFailed,
    BackupChatImportFailed,
    HtmlExportFailed,
    ExportLocationUnavailable,
}

impl PublicError {
    fn message(self) -> &'static str {
        match self {
            Self::SelectedSourceUnreadable => "Could not read the selected local source.",
            Self::SelectedSourceImportFailed => "Could not import the selected local source.",
            Self::AttachmentPreviewFailed => "Could not read media from the selected local source.",
            Self::BackupChatListFailed => "Could not read chats from the selected iPhone backup.",
            Self::BackupChatImportFailed => "Could not open the selected iPhone backup chat.",
            Self::HtmlExportFailed => "Could not write the HTML export.",
            Self::ExportLocationUnavailable => "Could not use the selected export location.",
        }
    }

    pub(crate) fn redact(self, _error: impl std::fmt::Display) -> String {
        self.message().to_owned()
    }
}
