# Network Room awareness and read-only task follow

This slice adds three per-client awareness tools to the native Network Room.

## Activity since last seen

The Network Room header and Activity tab count retained shared activity with a newer network sequence than the viewer's in-memory seen cursor. The badge remains visible while the panel is collapsed; opening Activity advances that cursor. Search matches activity text or member name; member and task filters narrow the retained feed, and task IDs present in activity details link to the shared task card. Public activity still excludes message bodies and private-message metadata. The cursor is scoped to the current app session and is not restored after leaving or restarting the Room.

## Due reminders

Each client can select a 5, 15 or 60 minute reminder window for the shared countdown, unfinished task due times and open milestone due times. Notices are deduplicated per deadline while the Network Room panel remains open. They are local in-panel notices; they do not send notifications when the app or panel is closed, and reminder choices are not persisted or synchronized.

## Consent-based task focus

A member can explicitly share one unfinished shared task ID as their current Room focus, or stop sharing it. Other joined members can choose to follow that person in a separate read-only panel. The panel shows the shared task summary, deadline and a few shared evidence records. Following never changes the viewer's task selection, editor navigation, open files or local state. Focus is cleared on leave/revocation or when the task completes. The wire payload contains only actor and task IDs; it does not contain a file path, cursor, buffer or editor selection.

This is task-level awareness, not file/cursor spectating, edit access or co-editing. The data remains subject to the existing self-hosted Room host visibility model.

## Explicit saved-file location sharing (protocol v3)

A member can manually share one workspace-relative saved-file path and positive line number up to 10 million, then update or clear it. The server rejects absolute, traversal, hidden and oversized paths, invalid line numbers and malformed snapshots. Locations are visible to all joined members, cleared when their member leaves or is revoked, and use the same authenticated invite/session boundary as other shared Room state. A viewer can follow one member in a separate read-only location preview and stop following at any time. Following does not itself open a file or move the local caret. The viewer can explicitly choose **Open in my workspace** to resolve the path against their own current workspace and move the caret to the shared line.

Chat messages can also attach one explicitly entered path and line, for a public Room message or the selected DM. The target inherits the message audience, so a third client never receives a private message's text or attached reference. Mentions and code references are separate message modes. Receivers can explicitly choose **Open in my workspace** for a reference. It resolves against the receiver's current workspace only; receiving or following a reference never opens a file automatically.

The feature shares path and line metadata only. It does not read or validate that path against the sender's disk, transmit file contents, unsaved buffers, cursor selection, diagnostics, or edits. Local navigation rejects absolute paths, traversal, streams, links/junctions, sensitive credential/repository paths, missing files and invalid lines. It opens only a regular file inside the receiver's current workspace and moves that editor caret; it does not alter the file. Receivers need their own matching workspace copy.

Independent-process tests check public location delivery, opt-out/leave cleanup, and public plus private anchored messages with a third client denied the DM and reference. Traversal/hidden-path and line rejection are covered. Windows CI and desktop acceptance remain separate; two-machine LAN and visible UI trials are still required.

## Verification boundaries

Core compatibility/validation and an independent-process two-client trial cover opt-in, recipient visibility, invalid task rejection, clearing and payload shape. App tests cover unread sequence counting and reminder window/deduplication. Windows CI can establish source/test/package results; visible desktop use and a two-machine LAN session remain separate acceptance gates.
