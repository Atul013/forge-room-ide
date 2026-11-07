export type RoomSummaryTask = { title: string; owner: string; status: 'open' | 'inProgress' | 'done'; priority?: string };
export type RoomSummaryEvent = { id: string; at: string; kind: string; name: string; recipient?: string; recipientId?: string };

const visibleKinds = new Set(['join', 'leave', 'task', 'agent', 'commit', 'merge', 'test', 'review', 'handoff', 'decision', 'conflict', 'revoke']);

function safeText(value: string, max: number): string {
	return value.replace(/[\u0000-\u001f\u007f]/g, ' ').replace(/\s+/g, ' ').trim().slice(0, max).replace(/([\\`*_{}\[\]()#+\-.!|>])/g, '\\$1');
}

/** Export a compact metadata-only timeline summary. Event details, messages, code, paths and patches are never read. */
export function formatRoomActivitySummary(project: string, generatedAt: string, tasks: readonly RoomSummaryTask[], events: readonly RoomSummaryEvent[]): string {
	const validTasks = tasks.filter(task => task && ['open', 'inProgress', 'done'].includes(task.status));
	const recent = events
		.filter(event => event && visibleKinds.has(event.kind) && !event.recipient && !event.recipientId && typeof event.at === 'string' && Number.isFinite(Date.parse(event.at)))
		.slice()
		.sort((left, right) => Date.parse(right.at) - Date.parse(left.at) || right.id.localeCompare(left.id))
		.slice(0, 30);
	const done = validTasks.filter(task => task.status === 'done').length;
	const active = validTasks.filter(task => task.status === 'inProgress').length;
	const open = validTasks.filter(task => task.status === 'open').length;
	const activity = recent.length ? recent.map(event => `- ${new Date(event.at).toISOString()} — ${safeText(event.name, 40) || 'Room member'} — ${safeText(event.kind, 24)}\n`).join('') : '- No eligible room activity recorded.\n';
	const taskList = validTasks.length ? validTasks.slice(0, 50).map(task => `- ${safeText(task.title, 100) || 'Untitled task'} — ${safeText(task.status, 20)} — ${safeText(task.owner, 40) || 'unassigned'}${task.priority ? ` — ${safeText(task.priority, 16)} priority` : ''}\n`).join('') : '- No room tasks recorded.\n';
	return `# Forge Room Activity Summary\n\n- **Project:** ${safeText(project, 120) || 'Untitled workspace'}\n- **Generated:** ${generatedAt}\n- **Tasks:** ${validTasks.length} total; ${done} done, ${active} in progress, ${open} open\n- **Timeline:** ${recent.length} eligible recent room events (maximum 30)\n\n## Recent room activity\n\n${activity}\n## Room tasks (${Math.min(validTasks.length, 50)} of ${validTasks.length})\n\n${taskList}\n## Privacy scope\n\nThis local summary includes task titles, owners, statuses, priority labels, event times, participant display names, and event categories. It excludes direct messages, message and note text, code, file paths, Git patches, invite secrets, and event details. It is not sent to room participants.\n`;
}
