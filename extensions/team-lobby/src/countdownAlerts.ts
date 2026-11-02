export const countdownMilestones = [60, 30, 10, 5, 0].map(minutes => minutes * 60_000);

/** Choose the next one-time reminder without replaying milestones already passed. */
export function nextCountdownMilestone(remainingMs: number, delivered: ReadonlySet<number>): number | undefined {
	if (!Number.isFinite(remainingMs)) { return undefined; }
	if (remainingMs <= 0) { return delivered.has(0) ? undefined : 0; }
	const upcoming = countdownMilestones.filter(value => value > 0 && value <= remainingMs && !delivered.has(value));
	if (upcoming.length) { return Math.max(...upcoming); }
	const recentlyPassed = countdownMilestones.filter(value => value > remainingMs && !delivered.has(value));
	return recentlyPassed.length ? Math.min(...recentlyPassed) : undefined;
}

export function countdownMilestoneLabel(milestoneMs: number): string {
	if (milestoneMs === 0) { return 'deadline reached'; }
	return `${milestoneMs / 60_000}-minute reminder`;
}

export function countdownMilestonesToRecord(milestoneMs: number, remainingMs: number): number[] {
	if (remainingMs <= 0) { return [...countdownMilestones]; }
	return countdownMilestones.filter(value => value === milestoneMs || value > remainingMs && value > 0 && value <= milestoneMs);
}

export function countdownAlertLabel(milestoneMs: number, remainingMs: number): number {
	if (remainingMs <= 0) { return 0; }
	const passed = countdownMilestonesToRecord(milestoneMs, remainingMs).filter(value => value > remainingMs && value > 0);
	return passed.length ? Math.min(...passed) : milestoneMs;
}

export function formatCountdownRemaining(remainingMs: number): string {
	if (remainingMs <= 0) { return 'deadline reached'; }
	const minutes = Math.ceil(remainingMs / 60_000);
	if (minutes < 60) { return `${minutes}m remaining`; }
	const hours = Math.floor(minutes / 60);
	const remainder = minutes % 60;
	return remainder ? `${hours}h ${remainder}m remaining` : `${hours}h remaining`;
}
