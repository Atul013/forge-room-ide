export type WorkspaceTestTask = { name: string; source: string };

function safeLabel(value: unknown, maxLength = 160): string {
	if (typeof value !== 'string') { return ''; }
	return value.replace(/[\u0000-\u001f\u007f]/g, ' ').replace(/\s+/g, ' ').trim().slice(0, maxLength);
}

export function normalizedWorkspaceTestTask(task: WorkspaceTestTask): WorkspaceTestTask {
	return { name: safeLabel(task.name), source: safeLabel(task.source, 80) };
}

export function matchingWorkspaceTestTasks<T extends WorkspaceTestTask>(tasks: T[], name: string, source: string): T[] {
	const wanted = normalizedWorkspaceTestTask({ name, source });
	return tasks.filter(task => {
		const candidate = normalizedWorkspaceTestTask(task);
		return candidate.name === wanted.name && candidate.source === wanted.source;
	});
}

/** Format registered test task names only. Never expose execution definitions or arguments. */
export function formatWorkspaceTestTasks(tasks: WorkspaceTestTask[], maxTasks = 100, maxCharacters = 10_000): string {
	const lines: string[] = [];
	for (const task of tasks.slice(0, Math.max(0, maxTasks))) {
		const name = safeLabel(task.name);
		const source = safeLabel(task.source, 80);
		if (name) { lines.push(`- ${name}${source ? ` (provider: ${source})` : ''}`); }
	}
	const header = lines.length ? `Registered test tasks (${lines.length}${tasks.length > lines.length ? ', additional tasks omitted' : ''}):` : 'No registered test tasks were found in the selected workspace.';
	const text = [header, ...lines].join('\n');
	if (text.length <= maxCharacters) { return text; }
	const suffix = '\n[truncated]';
	return `${text.slice(0, Math.max(0, maxCharacters - suffix.length))}${suffix}`.slice(0, maxCharacters);
}
