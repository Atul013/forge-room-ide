import { createHash } from 'node:crypto';
import { isStoredTaskDeadline } from './taskDeadline';

export const taskDeadlineMilestones = [15, 5, 0].map(minutes => minutes * 60_000);

/** Return the next reminder, coalescing missed reminders after sleep. */
export function nextTaskDeadlineMilestone(remainingMs: number, delivered: ReadonlySet<number>): number | undefined {
	if (!Number.isFinite(remainingMs)) { return undefined; }
	if (remainingMs <= 0) { return delivered.has(0) ? undefined : 0; }
	const upcoming = taskDeadlineMilestones.filter(value => value > 0 && value <= remainingMs && !delivered.has(value));
	if (upcoming.length) { return Math.max(...upcoming); }
	const passed = taskDeadlineMilestones.filter(value => value > remainingMs && !delivered.has(value));
	return passed.length ? Math.min(...passed) : undefined;
}

export function taskDeadlineMilestonesToRecord(milestoneMs: number, remainingMs: number): number[] {
	if (remainingMs <= 0) { return [...taskDeadlineMilestones]; }
	return taskDeadlineMilestones.filter(value => value === milestoneMs || value > remainingMs && value > 0 && value <= milestoneMs);
}

export function taskDeadlineMilestoneLabel(milestoneMs: number): string {
	if (milestoneMs === 0) { return 'deadline reached'; }
	return `${milestoneMs / 60_000}-minute reminder`;
}

export function taskDeadlineTriggerAt(dueAtMs: number, milestoneMs: number, now = Date.now()): number | undefined {
	if (!Number.isFinite(dueAtMs) || !Number.isFinite(now) || !taskDeadlineMilestones.includes(milestoneMs)) { return undefined; }
	return Math.max(now, dueAtMs - milestoneMs);
}

export function validTaskDeadlineAlertEntry(value: unknown): value is { key: string; milestones: number[] } {
	const entry = value as { key?: unknown; milestones?: unknown } | null;
	return !!entry && typeof entry.key === 'string' && /^[0-9a-f]{64}$/.test(entry.key) && Array.isArray(entry.milestones) && entry.milestones.every(mark => taskDeadlineMilestones.includes(mark));
}

export function taskDeadlineAlertKey(roomId: string, taskId: string, dueAt: string): string | undefined {
	if (!roomId || !taskId || !isStoredTaskDeadline(dueAt)) { return undefined; }
	return createHash('sha256').update(`${roomId}\n${taskId}\n${dueAt}`).digest('hex');
}
