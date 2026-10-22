import { createHash } from 'node:crypto';

export type StoredAcpSession = { sessionId: string; authMethodId?: string; updatedAt: number };

/** Keep an external session local to one workspace, ACP command, mode and argument set. */
export function acpSessionStorageKey(workspace: string, command: string, args: string[], mode: 'Ask' | 'Plan'): string {
	const identity = JSON.stringify({ workspace, command, args, mode });
	return `teamLobby.acpSession.v1.${createHash('sha256').update(identity).digest('hex')}`;
}

/** Validate untrusted extension workspace-state data before handing an ID to a provider. */
export function normalizeStoredAcpSession(value: unknown): StoredAcpSession | undefined {
	if (!value || typeof value !== 'object' || Array.isArray(value)) { return undefined; }
	const item = value as Record<string, unknown>;
	if (typeof item.sessionId !== 'string' || !item.sessionId.trim() || item.sessionId.length > 500 || /[\u0000-\u001f\u007f]/.test(item.sessionId)) { return undefined; }
	if (item.authMethodId !== undefined && (typeof item.authMethodId !== 'string' || item.authMethodId.length > 200 || /[\u0000-\u001f\u007f]/.test(item.authMethodId))) { return undefined; }
	if (!Number.isSafeInteger(item.updatedAt) || (item.updatedAt as number) < 0) { return undefined; }
	return { sessionId: item.sessionId, authMethodId: item.authMethodId as string | undefined, updatedAt: item.updatedAt as number };
}
