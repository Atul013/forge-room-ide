export type CompletionTask = { status: string; acceptanceCriteria?: string[]; completedCriteria?: string[] };

export function isValidCompletedCriteria(value: unknown, criteria: readonly string[] = []): value is string[] | undefined {
	if (value === undefined) { return true; }
	return Array.isArray(value)
		&& value.length <= 8
		&& value.every(item => typeof item === 'string' && criteria.includes(item))
		&& new Set(value).size === value.length;
}

export function taskCompletionDenial(current: CompletionTask | undefined, next: CompletionTask): string | undefined {
	const criteria = next.acceptanceCriteria || [];
	const completed = next.completedCriteria || [];
	if (current?.status === 'done' && next.status !== 'open'
		&& (JSON.stringify(current.acceptanceCriteria || []) !== JSON.stringify(criteria)
			|| JSON.stringify(current.completedCriteria || []) !== JSON.stringify(completed))) {
		return 'Reopen the completed task before changing its acceptance criteria or confirmations.';
	}
	if (!isValidCompletedCriteria(next.completedCriteria, criteria)) { return 'Completion confirmations must match the task acceptance criteria.'; }
	if (next.status !== 'done' && completed.length) { return 'Clear acceptance confirmations when reopening or restarting a task.'; }
	if (next.status === 'done' && current?.status !== 'done') {
		const missing = [...new Set(criteria)].filter(item => !completed.includes(item));
		if (missing.length) {
			return `Confirm every acceptance criterion before completing this task (${missing.length} remaining).`;
		}
	}
	return undefined;
}
