# Native Git conflict review

Source Control can open a section-by-section review for a text file with Git conflict markers. Each section shows the current and incoming labels and offers three explicit choices: keep current, keep incoming, or keep both. For diff3 files, the base is available in the comparison view. Text outside conflict sections is retained as-is.

The review does not modify the file until every section has a choice and the user selects **Save resolved file and stage**. Before writing, Forge Room checks that the workspace-relative path is safe, the file is still a regular file, the exact source text still matches the reviewed version, and the path remains unmerged in Git. It refuses to stage a result that still has conflict markers. Files over 10 MiB are outside this review flow; resolve those in an external editor and refresh Source Control afterward.

When resolution succeeds, the app writes the selected result, stages that one path, refreshes Git status, and reloads a clean open tab. A dirty open tab blocks opening the review so unsaved text cannot be replaced. If the file changes while the review is open, the user must reopen the review and choose again.

Automated parser tests cover multiple hunks, diff3 base sections, both-side choices, malformed markers and final-newline preservation. A real temporary Git repository test covers stale-source refusal, unsafe-path refusal, write-and-stage behavior, and the resulting clean resolution state. This is source/test evidence; a visible Windows conflict-resolution interaction remains a separate acceptance step.
