export type DirectMessageEvent = { kind: string; recipient?: string; recipientId?: string; senderId?: string };

export function canDeliverMessageEvent(event: DirectMessageEvent, participantId: string) {
	if (event.kind !== 'message' || !event.recipient) { return true; }
	if (!event.recipientId || !event.senderId) { return false; }
	return event.recipientId === participantId || event.senderId === participantId;
}

export function directMessagePeers<T extends { id: string }>(peers: T[], recipientId?: string, senderId?: string) {
	if (!recipientId) { return peers; }
	return peers.filter(peer => peer.id === recipientId || peer.id === senderId);
}
