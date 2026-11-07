export type ExportableRoomEvent = { kind?: unknown; recipientId?: unknown; senderId?: unknown };

/** Removes direct messages and their reactions unless the user explicitly includes them in a room export. */
export function roomHistoryForExport<T extends ExportableRoomEvent>(events: readonly T[], includeDirectMessages: boolean): T[] {
	return events.filter(event => includeDirectMessages || !(['message', 'reaction'].includes(String(event.kind)) && (typeof event.recipientId === 'string' || typeof event.senderId === 'string'))).map(event => ({ ...event }));
}
