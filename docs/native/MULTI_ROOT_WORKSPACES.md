# Native multi-root workspaces

Forge Room can keep one primary project folder and up to fifteen additional folders in Explorer. Use **Add Root** to include another folder; use a root's context menu to remove a secondary folder. The primary root cannot be removed independently. Duplicate, nested, missing and non-directory roots are rejected, so one file is not searched twice through overlapping roots.

Explorer, Quick Open, Find in Files and Replace All operate across every configured root. Quick Open prefixes each result with its root name and parent when names collide, discovers files on a worker thread, and caps the combined list at 20,000 entries so opening the palette does not block the UI on a large workspace. Replaced workspace scans can be cancelled. Find results remain absolute local paths. Replace All retains its explicit second-click confirmation and skips dirty open files; each file write keeps the existing stale-file and atomic-write checks.

The folder list is saved with the native window session. Older session files remain compatible because the new `folders` field is optional. Missing secondary folders are discarded on restore, and the primary folder remains the existing `folder` field.

The first root is the active project scope for Git, task pipelines, language servers, AI context/provider requests and Network Room. Use **Use as active project** in a secondary root's context menu to switch. The active project is labelled in Explorer and persisted first in the session. All roots and open documents (including dirty buffers) remain available. Switching uses the existing folder trust checks, resets language/debugger sessions, selects the new project AI conversation and reloads its local Room; the previous Network Room connection ends. Existing terminals retain their original cwd; any new terminal follows the active project. Secondary roots are not implicitly included in provider context.

**File > Save Workspace As** creates a versioned `.forge-workspace` JSON file containing only the ordered folder list. Paths below the document's directory are relative, so a project group can move together; outside paths stay absolute. Save is create-only: choose a new filename to preserve an existing document. No trust grants, provider credentials, editor buffers, terminal commands or Room identities are exported. **File > Open Workspace** (also available in the command palette) validates the complete document before changing the workspace. Unknown schema fields/versions, oversized documents, duplicate/nested roots and missing folders are rejected. The first root goes through the normal trust flow.

Explorer keyboard navigation traverses every configured root. Copy Relative Path resolves against the containing root, including secondary projects.

## Acceptance evidence

Local app unit tests cover root deduplication/removal and primary-root protection, duplicate root labels, and session persistence of existing secondary roots. App test results, CI, package/install checks, and visible desktop interaction are separate evidence. Physical two-client Room acceptance is unrelated to multi-root workspace behavior.
