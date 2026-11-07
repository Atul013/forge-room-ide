export const roomReactionChoices = ['👍', '❤️', '😂', '🎉', '👀'] as const;
export type RoomReaction = typeof roomReactionChoices[number];

export function isRoomReaction(value: unknown): value is RoomReaction {
	return typeof value === 'string' && (roomReactionChoices as readonly string[]).includes(value);
}

export function validIncomingRoomReaction(value: any): boolean {
	return value?.kind === 'reaction' && typeof value.reactionTo === 'string' && /^[0-9a-f-]{36}$/i.test(value.reactionTo) && isRoomReaction(value.reaction) && value.detail === value.reaction && value.file === undefined && value.line === undefined && value.anchor === undefined && value.recipient === undefined && value.recipientId === undefined && value.senderId === undefined && value.task === undefined && value.taskId === undefined && value.note === undefined && value.patch === undefined && value.reactionById === undefined && value.reactionActive === undefined;
}

export function validStoredRoomReaction(value: any): boolean {
	return value?.kind === 'reaction' && typeof value.reactionTo === 'string' && /^[0-9a-f-]{36}$/i.test(value.reactionTo) && isRoomReaction(value.reaction) && value.detail === value.reaction && typeof value.reactionById === 'string' && /^[0-9a-f-]{36}$/i.test(value.reactionById) && typeof value.reactionActive === 'boolean' && (value.recipientId === undefined && value.senderId === undefined && value.recipient === undefined || typeof value.recipientId === 'string' && value.recipientId.length <= 100 && typeof value.senderId === 'string' && value.senderId.length <= 100 && (value.recipient === undefined || typeof value.recipient === 'string' && value.recipient.length <= 40));
}

export type RoomReactionEvent = { kind: string; reactionTo?: string; reaction?: unknown; reactionById?: string; reactionActive?: boolean };

export function roomReactionState(events: readonly RoomReactionEvent[], messageId: string, viewerId: string) {
	const active = new Map<string, Set<string>>();
	for (const event of events) {
		if (event.kind !== 'reaction' || event.reactionTo !== messageId || !isRoomReaction(event.reaction) || !event.reactionById || typeof event.reactionActive !== 'boolean') { continue; }
		let participants = active.get(event.reaction);
		if (!participants) { participants = new Set(); active.set(event.reaction, participants); }
		if (event.reactionActive) { participants.add(event.reactionById); }
		else { participants.delete(event.reactionById); }
	}
	return roomReactionChoices.map(reaction => {
		const participants = active.get(reaction) || new Set<string>();
		return { reaction, count: participants.size, active: participants.has(viewerId) };
	});
}
