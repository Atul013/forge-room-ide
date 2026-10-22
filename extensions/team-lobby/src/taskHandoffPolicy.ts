export type HandoffTask = { id: string; status: string; ownerId?: string };

export function taskHandoffDenial(actorId: string, task: HandoffTask | undefined, targetParticipantId: string, connectedParticipantIds: ReadonlySet<string>, actorIsHost = false): string | undefined {
	if (!task) { return 'This room task no longer exists.'; }
	if (task.status === 'done') { return 'Completed tasks cannot be handed off. Reopen the task first.'; }
	if (task.ownerId && targetParticipantId === task.ownerId) { return 'Choose someone other than the current task owner.'; }
	if (!actorIsHost && (!task.ownerId || actorId !== task.ownerId)) { return 'Only the assigned task owner can request a handoff.'; }
	if (!targetParticipantId || targetParticipantId === actorId) { return 'Choose a different room participant for the handoff.'; }
	if (!connectedParticipantIds.has(targetParticipantId)) { return 'The new task owner must be connected to the room.'; }
	return undefined;
}
