export const taskPriorities = ['low', 'normal', 'high', 'critical'] as const;
export type TaskPriority = typeof taskPriorities[number];

export function isTaskPriority(value: unknown): value is TaskPriority {
	return typeof value === 'string' && taskPriorities.includes(value as TaskPriority);
}

export function taskPriorityRank(value: unknown): number {
	const rank = taskPriorities.indexOf(isTaskPriority(value) ? value : 'normal');
	return rank < 0 ? 1 : rank;
}

export function taskPriorityLabel(value: unknown): string {
	return isTaskPriority(value) ? value[0].toUpperCase() + value.slice(1) : 'Normal';
}
