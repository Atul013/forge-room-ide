//! Every action reachable from the command palette, the menus and shortcuts.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Command {
    NewFile,
    OpenFile,
    OpenFolder,
    Save,
    SaveAs,
    CloseTab,
    ReopenClosedTab,
    GoToSymbol,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    SelectNextOccurrence,
    GoToMatchingBracket,
    SelectToMatchingBracket,
    TransformUppercase,
    TransformLowercase,
    SortLinesAscending,
    SortLinesDescending,
    RemoveDuplicateLines,
    TrimTrailingWhitespace,
    JoinLines,
    CloseSavedTabs,
    NextTab,
    ToggleSidebar,
    ShowExplorer,
    ShowSourceControl,
    ShowRoom,
    ShowLobby,
    TogglePreview,
    ShowSearch,
    ColorTheme,
    Find,
    Replace,
    GoToLine,
    QuickOpen,
    GitRefresh,
    ToggleAutoSave,
    CopyPath,
    CopyRelativePath,
    RevealInExplorer,
    GitSync,
    SwitchBranch,
    MergeBranch,
    DeleteBranch,
    CreateTag,
    DeleteTag,
    AbortOperation,
    ToggleBlame,
    GitPull,
    GitPush,
    GitFetch,
    GitStash,
    GitStashApply,
    ToggleTerminal,
}

/// (command, title shown in the palette, shortcut hint)
pub const ALL: &[(Command, &str, &str)] = &[
    (Command::NewFile, "File: New File", "Ctrl+N"),
    (Command::OpenFile, "File: Open File…", "Ctrl+O"),
    (Command::OpenFolder, "File: Open Folder…", "Ctrl+Shift+O"),
    (Command::Save, "File: Save", "Ctrl+S"),
    (Command::SaveAs, "File: Save As…", "Ctrl+Shift+S"),
    (Command::CloseTab, "View: Close Tab", "Ctrl+W"),
    (
        Command::ReopenClosedTab,
        "View: Reopen Closed Tab",
        "Ctrl+Shift+T",
    ),
    (Command::CloseSavedTabs, "View: Close Saved Tabs", ""),
    (Command::GoToSymbol, "Go to Symbol in File…", "Ctrl+Shift+Y"),
    (Command::ZoomIn, "View: Increase Editor Font Size", "Ctrl+="),
    (
        Command::ZoomOut,
        "View: Decrease Editor Font Size",
        "Ctrl+-",
    ),
    (Command::ZoomReset, "View: Reset Editor Font Size", "Ctrl+0"),
    (
        Command::GoToMatchingBracket,
        "Selection: Go to Matching Bracket",
        "Ctrl+Shift+\\",
    ),
    (
        Command::SelectToMatchingBracket,
        "Selection: Select to Matching Bracket",
        "",
    ),
    (
        Command::SelectNextOccurrence,
        "Selection: Select Next Occurrence",
        "Ctrl+D",
    ),
    (
        Command::TransformUppercase,
        "Edit: Transform to Uppercase",
        "",
    ),
    (
        Command::TransformLowercase,
        "Edit: Transform to Lowercase",
        "",
    ),
    (
        Command::SortLinesAscending,
        "Edit: Sort Lines Ascending",
        "",
    ),
    (
        Command::SortLinesDescending,
        "Edit: Sort Lines Descending",
        "",
    ),
    (
        Command::RemoveDuplicateLines,
        "Edit: Remove Duplicate Lines",
        "",
    ),
    (
        Command::TrimTrailingWhitespace,
        "Edit: Trim Trailing Whitespace",
        "",
    ),
    (Command::JoinLines, "Edit: Join Lines", ""),
    (Command::NextTab, "View: Next Tab", "Ctrl+Tab"),
    (Command::QuickOpen, "Go to File…", "Ctrl+P"),
    (Command::GoToLine, "Go to Line…", "Ctrl+G"),
    (Command::Find, "Edit: Find", "Ctrl+F"),
    (Command::Replace, "Edit: Replace", "Ctrl+H"),
    (Command::ShowSearch, "Search: Find in Files", "Ctrl+Shift+F"),
    (Command::ShowExplorer, "View: Show Explorer", "Ctrl+Shift+E"),
    (
        Command::ShowSourceControl,
        "View: Show Source Control",
        "Ctrl+Shift+G",
    ),
    (Command::ShowRoom, "View: Show Room", "Ctrl+Shift+R"),
    (Command::ShowLobby, "View: Show Room Lobby", ""),
    (Command::TogglePreview, "View: Toggle Preview", ""),
    (Command::ToggleSidebar, "View: Toggle Sidebar", "Ctrl+B"),
    (Command::ToggleTerminal, "View: Toggle Terminal", "Ctrl+`"),
    (Command::ColorTheme, "Preferences: Color Theme", ""),
    (Command::GitRefresh, "Git: Refresh", ""),
    (Command::ToggleAutoSave, "File: Toggle Auto Save", ""),
    (Command::CopyPath, "File: Copy Path of Active File", ""),
    (
        Command::CopyRelativePath,
        "File: Copy Relative Path of Active File",
        "",
    ),
    (
        Command::RevealInExplorer,
        "File: Reveal Active File in File Explorer",
        "",
    ),
    (Command::SwitchBranch, "Git: Checkout to…", ""),
    (Command::MergeBranch, "Git: Merge Branch…", ""),
    (Command::DeleteBranch, "Git: Delete Branch…", ""),
    (Command::CreateTag, "Git: Create Tag…", ""),
    (Command::DeleteTag, "Git: Delete Tag…", ""),
    (
        Command::AbortOperation,
        "Git: Abort Merge, Cherry-pick or Revert",
        "",
    ),
    (Command::ToggleBlame, "GitLens: Toggle Line Blame", ""),
    (Command::GitSync, "Git: Sync Changes", ""),
    (Command::GitPull, "Git: Pull", ""),
    (Command::GitPush, "Git: Push", ""),
    (Command::GitFetch, "Git: Fetch", ""),
    (Command::GitStash, "Git: Stash Changes", ""),
    (Command::GitStashApply, "Git: Apply Latest Stash", ""),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_two_commands_advertise_the_same_shortcut() {
        let mut seen: std::collections::HashMap<&str, &str> = std::collections::HashMap::new();
        for (_, title, keys) in ALL {
            if keys.is_empty() {
                continue;
            }
            if let Some(other) = seen.insert(keys, title) {
                panic!("{keys} is listed for both \"{other}\" and \"{title}\"");
            }
        }
    }

    #[test]
    fn titles_are_unique_and_every_command_is_listed_once() {
        let mut titles: Vec<&str> = ALL.iter().map(|(_, title, _)| *title).collect();
        titles.sort_unstable();
        titles.dedup();
        assert_eq!(titles.len(), ALL.len());
        let mut commands: Vec<String> = ALL
            .iter()
            .map(|(command, _, _)| format!("{command:?}"))
            .collect();
        commands.sort_unstable();
        commands.dedup();
        assert_eq!(commands.len(), ALL.len());
    }
}
