export type AgentPresence = {
	id: string;
	name: string;
	role: 'agent';
	ownerId: string;
	status: 'working' | 'review';
	taskId?: string;
};

const participantIdPattern = /^[0-9a-f-]{36}$/i;

export function isAgentPresence(value: unknown, ownerId?: string): value is AgentPresence {
	if (!value || typeof value !== 'object' || Array.isArray(value)) { return false; }
	const presence = value as Record<string, unknown>;
	return Object.keys(presence).every(key => ['id', 'name', 'role', 'ownerId', 'status', 'taskId'].includes(key))
		&& typeof presence.id === 'string' && participantIdPattern.test(presence.id)
		&& typeof presence.name === 'string' && presence.name.trim().length > 0 && presence.name.length <= 40
		&& presence.role === 'agent'
		&& typeof presence.ownerId === 'string' && participantIdPattern.test(presence.ownerId) && (!ownerId || presence.ownerId === ownerId)
		&& (presence.status === 'working' || presence.status === 'review')
		&& (presence.taskId === undefined || typeof presence.taskId === 'string' && presence.taskId.trim().length > 0 && presence.taskId.length <= 100 && !/[\r\n\0]/.test(presence.taskId));
}
