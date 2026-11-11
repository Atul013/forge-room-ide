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
    SelectNextOccurrence,
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
    GitSync,
    SwitchBranch,
    MergeBranch,
    DeleteBranch,
    AbortOperation,
    ToggleBlame,
    GitPull,
    GitPush,
    GitFetch,
    GitStash,
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
    (Command::SwitchBranch, "Git: Checkout to…", ""),
    (Command::MergeBranch, "Git: Merge Branch…", ""),
    (Command::DeleteBranch, "Git: Delete Branch…", ""),
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
];

#[cfg(test)]
mod tests {
    use super::*;

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
