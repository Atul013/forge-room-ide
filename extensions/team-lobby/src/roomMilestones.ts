import { isStoredTaskDeadline, parseFutureDeadlineInput } from './taskDeadline';

export const maxRoomMilestones = 20;
export const maxMilestoneLabelLength = 80;
export type RoomMilestone = { id: string; label: string; dueAt: string };
export type ParsedMilestone = { label: string; dueAt: string };

export function isRoomMilestones(value: unknown): value is RoomMilestone[] {
	if (!Array.isArray(value) || value.length > maxRoomMilestones) { return false; }
	const ids = new Set<string>();
	return value.every(item => {
		if (!item || typeof item.id !== 'string' || !/^[0-9a-f-]{36}$/i.test(item.id) || ids.has(item.id) || typeof item.label !== 'string' || !item.label.trim() || item.label !== item.label.trim() || item.label.length > maxMilestoneLabelLength || /[\u0000-\u001f\u007f]/.test(item.label) || !isStoredTaskDeadline(item.dueAt)) { return false; }
		ids.add(item.id);
		return true;
	});
}

/** Parse one `label | ISO 8601 date/time with timezone` milestone per line. */
export function parseRoomMilestonesInput(input: string, now = Date.now()): { milestones?: ParsedMilestone[]; error?: string } {
	const lines = input.split(/\r?\n/).map(line => line.trim()).filter(Boolean);
	if (lines.length > maxRoomMilestones) { return { error: `Enter no more than ${maxRoomMilestones} milestones.` }; }
	const milestones: ParsedMilestone[] = [];
	const labels = new Set<string>();
	for (const line of lines) {
		const separator = line.indexOf('|');
		if (separator < 1) { return { error: 'Use one milestone per line: label | 2026-10-25T18:00+05:30' }; }
		const label = line.slice(0, separator).trim();
		const parsed = parseFutureDeadlineInput(line.slice(separator + 1).trim(), now);
		if (!label || label.length > maxMilestoneLabelLength) { return { error: `Milestone labels must contain 1 to ${maxMilestoneLabelLength} characters.` }; }
		if (labels.has(label.toLocaleLowerCase())) { return { error: 'Milestone labels must be unique.' }; }
		if (parsed.error || !parsed.dueAt) { return { error: `${label}: ${parsed.error || 'Enter a future milestone time.'}` }; }
		labels.add(label.toLocaleLowerCase());
		milestones.push({ label, dueAt: parsed.dueAt });
	}
	milestones.sort((left, right) => Date.parse(left.dueAt) - Date.parse(right.dueAt) || left.label.localeCompare(right.label));
	return { milestones };
}

export function milestoneStatus(milestone: RoomMilestone, now = Date.now()): 'passed' | 'soon' | 'upcoming' {
	const remaining = Date.parse(milestone.dueAt) - now;
	return remaining < 0 ? 'passed' : remaining <= 60 * 60_000 ? 'soon' : 'upcoming';
}
