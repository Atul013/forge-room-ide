# Native multi-root workspaces

Forge Room can keep one primary project folder and up to fifteen additional folders in Explorer. Use **Add Root** to include another folder; use a root's context menu to remove a secondary folder. The primary root cannot be removed independently. Duplicate, nested, missing and non-directory roots are rejected, so one file is not searched twice through overlapping roots.

Explorer, Quick Open, Find in Files and Replace All operate across every configured root. Quick Open prefixes each result with its root name and parent when names collide. Find results remain absolute local paths. Replace All retains its explicit second-click confirmation and skips dirty open files; each file write keeps the existing stale-file and atomic-write checks.

The folder list is saved with the native window session. Older session files remain compatible because the new `folders` field is optional. Missing secondary folders are discarded on restore, and the primary folder remains the existing `folder` field.

The first root remains the primary project scope for Git, task pipelines, language servers, AI context/provider requests and Network Room. Secondary roots are not silently sent to an AI provider or shared with a Room. This release does not claim cross-root Git or project-tool routing.

## Acceptance evidence

Local app unit tests cover root deduplication/removal and primary-root protection, duplicate root labels, and session persistence of existing secondary roots. App test results, CI, package/install checks, and visible desktop interaction are separate evidence. Physical two-client Room acceptance is unrelated to multi-root workspace behavior.
