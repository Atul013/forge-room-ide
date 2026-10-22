export type RoomTaskContextItem = {
	id: string;
	title: string;
	owner: string;
	status: string;
	dependencies?: string[];
	acceptanceCriteria?: string[];
	dueAt?: string;
};

function clean(value: unknown, max: number): string {
	return typeof value === 'string' ? value.replace(/[\u0000-\u001f\u007f]/g, ' ').replace(/\s+/g, ' ').trim().slice(0, max) : '';
}

/** Format shared room task metadata for explicit AI transfer; never includes messages, notes, diffs, or file content. */
export function formatRoomTasksForModel(tasks: readonly RoomTaskContextItem[], maxTasks = 50, maxCharacters = 12_000): string {
	const limit = Math.max(0, Math.min(50, Math.floor(maxTasks)));
	const selected = tasks.slice(0, limit);
	if (!selected.length) { return 'No tasks are available in the active room.'; }
	const lines = [`Room tasks (${selected.length}${tasks.length > selected.length ? ', additional tasks omitted' : ''}):`];
	for (const task of selected) {
		const id = clean(task.id, 40);
		const title = clean(task.title, 180) || '(untitled task)';
		const owner = clean(task.owner, 100) || 'unassigned';
		const status = task.status === 'open' || task.status === 'inProgress' || task.status === 'done' ? task.status : 'unknown';
		const due = typeof task.dueAt === 'string' && !Number.isNaN(Date.parse(task.dueAt)) ? ` | due: ${new Date(task.dueAt).toISOString()}` : '';
		const dependencies = Array.isArray(task.dependencies) ? task.dependencies.slice(0, 20).map(value => clean(value, 40)).filter(Boolean) : [];
		lines.push(`- ${id || 'unknown-id'} — ${title} | status: ${status} | owner: ${owner}${due}${dependencies.length ? ` | depends on: ${dependencies.join(', ')}` : ''}`);
		const criteria = Array.isArray(task.acceptanceCriteria) ? task.acceptanceCriteria.slice(0, 10).map(value => clean(value, 240)).filter(Boolean) : [];
		if (criteria.length) { lines.push(...criteria.map(value => `  - ${value}`)); }
	}
	const output = lines.join('\n');
	if (output.length <= maxCharacters) { return output; }
	const suffix = '\n[truncated]';
	return `${output.slice(0, Math.max(0, maxCharacters - suffix.length))}${suffix}`.slice(0, maxCharacters);
}
