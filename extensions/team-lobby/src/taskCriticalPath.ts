export type EstimatedPathTask = { id: string; title: string; status: 'open' | 'inProgress' | 'done'; dependencies?: string[]; estimateMinutes?: number };
export type CriticalPath = { complete: boolean; taskIds: string[]; estimatedMinutes?: number; reason?: string };

/** Return one deterministic longest estimated dependency chain; incomplete estimates fail closed. */
export function analyzeTaskCriticalPath(tasks: readonly EstimatedPathTask[]): CriticalPath {
	const active = tasks.filter(task => task.status !== 'done');
	if (!active.length) { return { complete: true, taskIds: [], estimatedMinutes: 0 }; }
	if (active.some(task => !Number.isSafeInteger(task.estimateMinutes) || (task.estimateMinutes as number) < 1 || (task.estimateMinutes as number) > 10_080)) {
		return { complete: false, taskIds: [], reason: 'Add a valid estimate to every unfinished task.' };
	}
	const byId = new Map(tasks.map(task => [task.id, task]));
	const activeIds = new Set(active.map(task => task.id));
	const visiting = new Set<string>();
	const memo = new Map<string, { duration: number; taskIds: string[] }>();
	const visit = (id: string): { duration: number; taskIds: string[] } | undefined => {
		const cached = memo.get(id);
		if (cached) { return cached; }
		if (visiting.has(id)) { return undefined; }
		const task = byId.get(id);
		if (!task || !activeIds.has(id) || !Number.isSafeInteger(task.estimateMinutes)) { return undefined; }
		visiting.add(id);
		let longestDependency: { duration: number; taskIds: string[] } | undefined;
		for (const dependencyId of task.dependencies || []) {
			const dependency = byId.get(dependencyId);
			if (!dependency) { visiting.delete(id); return undefined; }
			if (dependency.status === 'done') { continue; }
			const dependencyPath = visit(dependencyId);
			if (!dependencyPath) { visiting.delete(id); return undefined; }
			if (!longestDependency || dependencyPath.duration > longestDependency.duration || dependencyPath.duration === longestDependency.duration && dependencyPath.taskIds.join('\0').localeCompare(longestDependency.taskIds.join('\0')) < 0) {
				longestDependency = dependencyPath;
			}
		}
		visiting.delete(id);
		const result = { duration: (task.estimateMinutes as number) + (longestDependency?.duration || 0), taskIds: [...(longestDependency?.taskIds || []), id] };
		memo.set(id, result);
		return result;
	};
	let longest: { duration: number; taskIds: string[] } | undefined;
	for (const task of active) {
		const candidate = visit(task.id);
		if (!candidate) { return { complete: false, taskIds: [], reason: 'Room task dependencies contain a cycle or unavailable task.' }; }
		if (!longest || candidate.duration > longest.duration || candidate.duration === longest.duration && candidate.taskIds.join('\0').localeCompare(longest.taskIds.join('\0')) < 0) { longest = candidate; }
	}
	return { complete: true, taskIds: longest?.taskIds || [], estimatedMinutes: longest?.duration || 0 };
}
