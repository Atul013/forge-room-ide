## Team Lobby 0.129.0 - local task deadline reminders

Task owners can enable local reminders for their unfinished room tasks at 15 minutes, 5 minutes, and expiry. The setting defaults off. Reminders are deduplicated per task and deadline, recover missed marks without replaying old reminders, and never create room events or send data.

**Verification:** Both Team Lobby copies pass TypeScript builds and 233/233 tests; webview syntax check passes. Nine-file VSIX SHA-256: `55D685464D3EB5DFE22C1E92CFB28B0A0DA61C94CA9A05462164F3154D71C10F`; six retained packages match, and three product manifests reference this version and hash. Disposable-profile VS Code installation succeeds. Live reminder timing and preference interaction remain unverified.

## Team Lobby 0.128.0 - explicit searchable room decisions

Participants can record a room decision with a title and rationale. The extension asks for confirmation before broadcasting it; the actor and timestamp are retained in the room event history. The host validates the bounded title/rationale format; valid decisions are included in searchable local history. Room protocol is 20 so clients that do not recognize this event type cannot join as if fully compatible.

**Verification:** Both TypeScript builds and 230/230 tests pass; webview syntax check passes. Nine-file VSIX SHA-256: 35B9F5CCD85F2DCE0A41A741C0F19839295776B59869D4EF841592B24612AC22; five retained packages and manifests match, and disposable-profile installation succeeds. Interactive multi-client decision display and search remain unverified.

## Team Lobby 0.127.0 - participant-local message search

Every connected participant can search up to 5,000 messages retained on their device, 25 newest matches per page. Search stays local and uses the same direct-message visibility filter as delivery. A new guest starts with recent room history and adds messages while connected. Protocol remains 19.

**Verification:** Both TypeScript builds and 228/228 tests pass; webview syntax check passes. Nine-file VSIX SHA-256: 2636CFEDDDBAC212F1A7596969A92108544766014FDF5CCE8A66FB1313497B95; five retained packages and manifests match, and disposable-profile installation succeeds. Interactive multi-client privacy remains unverified.

## Team Lobby 0.126.0 - host-local searchable message history

Hosts can search up to 5,000 locally retained messages, 25 newest matches per page. Search text stays local, direct-message matches are limited to eligible participants, and guests can search the recent room pulse only. Room protocol remains 19.

**Verification:** Both extension copies build and pass 227/227 tests. Webview syntax check passes. Nine-file VSIX SHA-256: 2157DB344199D0EB952E208030ED9C15261D4F3C5AED34A29CA2A31F9D915C11; five retained packages match, and disposable-profile installation succeeds. Interactive IDE search and guest behavior remain unverified.

## Team Lobby 0.125.0 - threaded room messages

Messages can open a thread and add replies. The host validates the parent and preserves public or original direct-message audience. Protocol is 19.

**Verification:** Both extension copies build and pass 225/225 tests. Webview syntax check passes. Nine-file VSIX SHA-256: 5F8734852BB7A625960F87E07B72AA4CEBD5729202BC753F1C77BF0D91652547; five retained packages match, and disposable-profile installation succeeds. Two-client thread privacy remains unverified.
