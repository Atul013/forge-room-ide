/** Reuse a persisted room agent ID, or create one for a legacy or invalid session. */
export function ensureAgentSessionId(current: string | undefined, createId: () => string): string {
	return current && /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i.test(current) ? current : createId();
}
