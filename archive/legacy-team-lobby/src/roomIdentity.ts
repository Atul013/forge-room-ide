export function stableParticipantId(memberId: string, selfParticipantId: string | undefined) {
	return memberId === 'self' && selfParticipantId ? selfParticipantId : memberId;
}

export function remapLegacySelfTaskOwner<T extends { ownerId?: string }>(task: T, selfParticipantId: string | undefined): T {
	if (task.ownerId !== 'self' || !selfParticipantId) { return task; }
	return { ...task, ownerId: selfParticipantId };
}
