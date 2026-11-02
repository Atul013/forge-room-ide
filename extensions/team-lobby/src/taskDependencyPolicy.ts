export type DependencyTask = { id: string; title?: string; status: string; dependencies?: string[] };

function dependsOn(tasks: ReadonlyMap<string, DependencyTask>, taskId: string, targetId: string, seen = new Set<string>()): boolean {
	if (taskId === targetId) { return true; }
	if (seen.has(taskId)) { return false; }
	seen.add(taskId);
	return (tasks.get(taskId)?.dependencies || []).some(id => dependsOn(tasks, id, targetId, seen));
}

/** Prevents starting/completing work before prerequisites, or reopening a prerequisite under active dependents. */
export function taskDependencyDenial(current: DependencyTask | undefined, next: DependencyTask, existing: readonly DependencyTask[]): string | undefined {
	const tasks = new Map(existing.map(task => [task.id, task]));
	tasks.set(next.id, next);
	if (next.status !== 'open') {
		const blocked = (next.dependencies || []).filter(id => tasks.get(id)?.status !== 'done');
		if (blocked.length) {
			const names = blocked.slice(0, 3).map(id => tasks.get(id)?.title || id || 'unknown task');
			return `Task is blocked until its dependencies are done: ${names.join(', ')}${blocked.length > names.length ? ', and more' : ''}.`;
		}
	}
	if (current && current.status !== 'open' && next.status === 'open') {
		const activeDependents = [...tasks.values()].filter(task => task.id !== next.id && task.status !== 'open' && dependsOn(tasks, task.id, next.id));
		if (activeDependents.length) {
			const names = activeDependents.slice(0, 3).map(task => task.title || task.id);
			return `Cannot reopen this task while dependent work is active: ${names.join(', ')}${activeDependents.length > names.length ? ', and more' : ''}.`;
		}
	}
	return undefined;
}
