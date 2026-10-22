export type InviteIdentity = { name: string; role: 'human' | 'agent' };

/** Bind a participant ID to the first identity presented with a bearer invite. */
export function acceptInviteIdentity(participants: Map<string, InviteIdentity>, participantId: string, identity: InviteIdentity) {
	const normalized = { name: identity.name.trim().slice(0, 40), role: identity.role };
	const existing = participants.get(participantId);
	if (existing) {
		return existing.name === normalized.name && existing.role === normalized.role ? { accepted: true, identity: existing } : { accepted: false as const };
	}
	participants.set(participantId, normalized);
	return { accepted: true, identity: normalized };
}
