export type ExportableRoomEvent = { kind?: unknown; recipientId?: unknown; senderId?: unknown };

/** Removes private messages unless the user explicitly includes them in a room export. */
export function roomHistoryForExport<T extends ExportableRoomEvent>(events: readonly T[], includeDirectMessages: boolean): T[] {
	return events.filter(event => includeDirectMessages || !(event.kind === 'message' && (typeof event.recipientId === 'string' || typeof event.senderId === 'string'))).map(event => ({ ...event }));
}
