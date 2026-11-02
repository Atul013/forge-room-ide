/** Validate a one-time, workspace-relative editor location share. */
export function validSharedLocation(file: unknown, line: unknown): file is string {
	if (typeof file !== 'string' || !file.trim() || file.length > 500 || typeof line !== 'number' || !Number.isSafeInteger(line) || line < 1 || line > 10_000_000) { return false; }
	const normalized = file.replace(/\\/g, '/');
	return !normalized.startsWith('/') && !/^[a-zA-Z]:/.test(normalized) && !/[\u0000-\u001f\u007f]/.test(normalized) && !normalized.split('/').some(part => !part || part === '.' || part === '..');
}

/** Throttle ephemeral editor-location updates to at most four per second. */
export function locationUpdateAllowed(lastSentAt: number | undefined, now: number): boolean {
	return Number.isFinite(now) && (lastSentAt === undefined || Number.isFinite(lastSentAt) && now - lastSentAt >= 250);
}

export type RoomEditorLocation = { participantId: string; name: string; role: 'human' | 'agent'; file: string; line: number; updatedAt: string };

/** Validate and reduce an ephemeral location wire message to its permitted fields. */
export function prepareEditorLocationUpdate(message: unknown, actor: { id: string; name: string; role: 'human' | 'agent' }, lastSentAt: number | undefined, now: number): { cleared: boolean; location?: RoomEditorLocation; lastSentAt?: number } {
	if (!message || typeof message !== 'object' || Array.isArray(message) || !Number.isFinite(now)) { return { cleared: false }; }
	const input = message as Record<string, unknown>;
	if (input.type !== undefined && input.type !== 'editorLocation' || input.version !== undefined && !Number.isInteger(input.version)) { return { cleared: false }; }
	if (input.active === false) {
		return Object.keys(input).every(key => ['active', 'type', 'version'].includes(key)) ? { cleared: true } : { cleared: false };
	}
	const file = input.file;
	const line = input.line;
	if (input.active !== true || !Object.keys(input).every(key => ['active', 'file', 'line', 'type', 'version'].includes(key)) || typeof file !== 'string' || typeof line !== 'number' || !validSharedLocation(file, line) || !locationUpdateAllowed(lastSentAt, now) || !/^[0-9a-f-]{36}$/i.test(actor.id)) { return { cleared: false }; }
	return { cleared: false, lastSentAt: now, location: { participantId: actor.id, name: actor.name.slice(0, 40), role: actor.role, file, line, updatedAt: new Date(now).toISOString() } };
}
