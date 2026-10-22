const inputPattern = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})(?::(\d{2})(?:\.(\d{1,3}))?)?(Z|([+-])(\d{2}):(\d{2}))$/;
export const defaultHackathonDurationMs = 24 * 60 * 60 * 1000;

export function deadlineAfterDuration(durationMs: number, now = Date.now()): string | undefined {
	const timestamp = now + durationMs;
	return Number.isSafeInteger(durationMs) && durationMs > 0 && Number.isFinite(timestamp) && Math.abs(timestamp) <= 8_640_000_000_000_000 ? new Date(timestamp).toISOString() : undefined;
}

export function parseTaskDeadlineInput(input: string): { dueAt?: string; error?: string } {
	const value = input.trim();
	if (!value) { return {}; }
	const match = inputPattern.exec(value);
	if (!match) { return { error: 'Use an ISO 8601 date and time with an explicit timezone, such as 2026-10-22T18:00+05:30' }; }
	const [, y, mo, d, h, mi, s = '0', , zone, , offsetHour = '0', offsetMinute = '0'] = match;
	const year = Number(y), month = Number(mo), day = Number(d), hour = Number(h), minute = Number(mi), second = Number(s);
	const offsetH = Number(offsetHour), offsetM = Number(offsetMinute);
	const date = new Date(Date.UTC(year, month - 1, day));
	if (month < 1 || month > 12 || day < 1 || date.getUTCFullYear() !== year || date.getUTCMonth() !== month - 1 || date.getUTCDate() !== day || hour > 23 || minute > 59 || second > 59 || offsetH > 23 || offsetM > 59) {
		return { error: 'Enter a real calendar date, time, and timezone offset' };
	}
	const timestamp = Date.parse(value);
	return Number.isFinite(timestamp) ? { dueAt: new Date(timestamp).toISOString() } : { error: 'Enter a valid ISO 8601 date and time' };
}

export function isStoredTaskDeadline(value: unknown): value is string {
	return typeof value === 'string' && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/.test(value) && Number.isFinite(Date.parse(value)) && new Date(value).toISOString() === value;
}

export function parseFutureDeadlineInput(input: string, now = Date.now()): { dueAt?: string; error?: string } {
	const parsed = parseTaskDeadlineInput(input);
	if (parsed.error || !parsed.dueAt) { return parsed; }
	return Date.parse(parsed.dueAt) > now ? parsed : { error: 'The hackathon countdown must end in the future' };
}
