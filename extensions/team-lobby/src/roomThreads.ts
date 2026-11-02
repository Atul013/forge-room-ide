export type ThreadMessageRef = { id: string; kind: string; recipientId?: string; senderId?: string; replyToId?: string };

/** Enforce reply scope at the room host before recording a threaded message. */
export function threadReplyDenial(replyToId: string | undefined, events: readonly ThreadMessageRef[], actorId: string, recipientId?: string): string | undefined {
	if (!replyToId) { return undefined; }
	const selected = events.find(event => event.id === replyToId && event.kind === 'message');
	const parent = selected?.replyToId ? events.find(event => event.id === selected.replyToId && event.kind === 'message') : selected;
	if (!parent) { return 'The message this reply references is no longer available.'; }
	const isPrivate = !!parent.recipientId || !!parent.senderId;
	if (!isPrivate) { return recipientId ? 'Replies to room messages must stay visible in the room.' : undefined; }
	if (!parent.recipientId || !parent.senderId || actorId !== parent.recipientId && actorId !== parent.senderId) { return 'Only participants in the original direct-message conversation can reply.'; }
	const expectedRecipient = parent.senderId === actorId ? parent.recipientId : parent.senderId;
	return recipientId === expectedRecipient ? undefined : 'A direct-message reply must stay within the original conversation.';
}

export function threadMessages<T extends ThreadMessageRef>(events: readonly T[], rootId: string): T[] {
	const root = events.find(event => event.id === rootId && event.kind === 'message');
	if (!root) { return []; }
	return events.filter(event => event.id === rootId || (event.kind === 'message' && event.replyToId === rootId));
}
