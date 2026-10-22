# Team Lobby IDE

First runnable slice of the desktop IDE described in the **Build Your IDE** conversation. It runs as a Code OSS / VS Code extension while the product is developed into a branded fork.

## Try it

1. Run `npm install` and `npm run build`.
2. Open this folder in VS Code and press **F5** (Extension Development Host).
3. Run **Team Lobby: Host Room** in one window. Copy the room address and secret shown in the notification.
4. In another Extension Development Host window, run **Team Lobby: Join Room**. Enter the address and secret.
5. Open **Team Lobby** in the activity bar. Use **Post Update** to send a message. Editing and saving files in the same workspace also creates feed events.

The host binds to all network interfaces, so teammates on a LAN can connect using its LAN IP. The room uses a secret, but traffic is plain WebSocket. Use it only on a trusted LAN during this prototype phase; internet rooms need TLS, identities, and access controls.

## Scope now

Human and agent participants use the same room protocol. A participant chooses a name and role on joining. The feed shows joins, leaves, posts, active file changes, and saves. It transmits file paths relative to the workspace, never file contents. The room currently lives only while the host editor is open; it does not store history.

## Next build slices

1. Persistent room service with authenticated internet access, reconnect, and history.
2. Shared task board and agent task events using the same protocol.
3. Diff and test evidence attached to changes, with review and handoff.
4. Integrate the extension into a branded Code OSS fork after the collaboration model is stable.

Code OSS source is MIT licensed; Microsoft's branded distribution and its marketplace have separate terms. See the [Code OSS differences](https://github.com/microsoft/vscode-wiki/blob/main/Differences-between-the-repository-and-Visual-Studio-Code.md) and [source build guide](https://github.com/microsoft/vscode/wiki/How-to-Contribute).
