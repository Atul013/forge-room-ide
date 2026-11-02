import { canDeliverMessageEvent } from './directMessages';

export type SearchableRoomMessage = { kind: string; name: string; detail: string; recipient?: string };

export function searchRoomMessages<T extends SearchableRoomMessage>(events: readonly T[], query: string, offset = 0, limit = 25): { results: T[]; total: number } {
	const needle = query.trim().slice(0, 100).toLocaleLowerCase();
	const pageSize = Math.min(50, Math.max(1, limit));
	const matches = events.filter(event => (event.kind === 'message' || event.kind === 'decision') && [event.name, event.detail, event.recipient].filter(Boolean).join(' ').toLocaleLowerCase().includes(needle)).slice().reverse();
	return { results: matches.slice(Math.max(0, offset), Math.max(0, offset) + pageSize), total: matches.length };
}

export function searchVisibleRoomMessages<T extends SearchableRoomMessage & { recipientId?: string; senderId?: string }>(events: readonly T[], participantId: string, query: string, offset = 0, limit = 25) {
	return searchRoomMessages(events.filter(event => canDeliverMessageEvent(event, participantId)), query, offset, limit);
}
