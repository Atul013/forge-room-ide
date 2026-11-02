export type AuthorizedTaskFields = { id: string; status: string; ownerId?: string; worktreeBranch?: string };

/** Host-side policy for remote room-task writes. Display names are never used as authority. */
export function taskWriteDenial(
	actorId: string,
	hostId: string,
	current: AuthorizedTaskFields | undefined,
	next: AuthorizedTaskFields,
	connectedOwnerIds: ReadonlySet<string>
): string | undefined {
	if (!next.ownerId) { return 'Task must have a participant owner ID.'; }
	if (!current) {
		if (next.status !== 'open' || next.worktreeBranch) { return 'New tasks must start open without a worktree branch.'; }
		if (!connectedOwnerIds.has(next.ownerId)) { return 'Task owner must be a connected room participant.'; }
		return undefined;
	}
	if (current.id !== next.id) { return 'Task ID cannot be changed.'; }
	const isHost = actorId === hostId;
	if (!isHost && (!current.ownerId || actorId !== current.ownerId)) { return 'Only the host or current task owner can update this task.'; }
	if (!isHost && next.ownerId !== current.ownerId) { return 'Only the host can reassign a task.'; }
	if (next.ownerId !== current.ownerId && !connectedOwnerIds.has(next.ownerId)) { return 'New task owner must be connected to the room.'; }
	if (!isHost && current.status === 'done' && next.status !== 'done') { return 'Only the host can reopen a completed task.'; }
	return undefined;
}
