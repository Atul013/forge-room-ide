export type RoomTaskContextItem = {
	id?: unknown;
	title?: unknown;
	owner?: unknown;
	status?: unknown;
	priority?: unknown;
	estimateMinutes?: unknown;
	dependencies?: unknown;
	acceptanceCriteria?: unknown;
	dueAt?: unknown;
};

function clean(value: unknown, max: number): string {
	return typeof value === 'string' ? value.replace(/[\u0000-\u001f\u007f]/g, ' ').replace(/\s+/g, ' ').trim().slice(0, max) : '';
}

/** Format shared room task metadata for explicit AI transfer; never includes messages, notes, diffs, or file content. */
export function formatRoomTasksForModel(tasks: readonly RoomTaskContextItem[], maxTasks = 50, maxCharacters = 12_000): string {
	const taskLimit = Number.isFinite(maxTasks) ? Math.max(0, Math.min(50, Math.floor(maxTasks))) : 50;
	const characterLimit = Number.isFinite(maxCharacters) ? Math.max(0, Math.min(12_000, Math.floor(maxCharacters))) : 12_000;
	const selected = tasks.slice(0, taskLimit);
	if (!selected.length) { return 'No tasks are available in the active room.'; }
	const lines = [`Room tasks (${selected.length}${tasks.length > selected.length ? ', additional tasks omitted' : ''}):`];
	for (const task of selected) {
		const id = clean(task.id, 40);
		const title = clean(task.title, 180) || '(untitled task)';
		const owner = clean(task.owner, 100) || 'unassigned';
		const status = task.status === 'open' || task.status === 'inProgress' || task.status === 'done' ? task.status : 'unknown';
		const priority = task.priority === 'low' || task.priority === 'normal' || task.priority === 'high' || task.priority === 'critical' ? task.priority : 'normal';
		const estimate = Number.isSafeInteger(task.estimateMinutes) && (task.estimateMinutes as number) >= 1 && (task.estimateMinutes as number) <= 10_080 ? ` | estimate: ${task.estimateMinutes} minutes` : '';
		const due = typeof task.dueAt === 'string' && !Number.isNaN(Date.parse(task.dueAt)) ? ` | due: ${new Date(task.dueAt).toISOString()}` : '';
		const dependencies = Array.isArray(task.dependencies) ? task.dependencies.slice(0, 20).map(value => clean(value, 40)).filter(Boolean) : [];
		lines.push(`- ${id || 'unknown-id'} - ${title} | status: ${status} | priority: ${priority} | owner: ${owner}${estimate}${due}${dependencies.length ? ` | depends on: ${dependencies.join(', ')}` : ''}`);
		const criteria = Array.isArray(task.acceptanceCriteria) ? task.acceptanceCriteria.slice(0, 10).map(value => clean(value, 240)).filter(Boolean) : [];
		if (criteria.length) { lines.push(...criteria.map(value => `  - ${value}`)); }
	}
	const output = lines.join('\n');
	if (output.length <= characterLimit) { return output; }
	const suffix = '\n[truncated]';
	return `${output.slice(0, Math.max(0, characterLimit - suffix.length))}${suffix}`.slice(0, characterLimit);
}
