export type RevocablePeer = { inviteToken: string };

export function revokeInvite<T extends RevocablePeer>(invites: Map<string, unknown>, peers: Map<string, T>, token: string) {
	const revoked = invites.delete(token);
	return { revoked, peers: revoked ? [...peers.values()].filter(peer => peer.inviteToken === token) : [] };
}
