import { taskPriorityLabel, taskPriorityRank, type TaskPriority } from './taskPriority';
import { analyzeTaskCriticalPath } from './taskCriticalPath';
import { formatTaskEstimate } from './taskEstimate';

export type TaskTriageSource = {
	id: string;
	title: string;
	owner: string;
	status: 'open' | 'inProgress' | 'done';
	dependencies?: string[];
	dueAt?: string;
	priority?: TaskPriority;
	estimateMinutes?: number;
};

export type TaskTriageItem = {
	task: TaskTriageSource;
	recommendation: string;
	blockers: string[];
	downstreamCount: number;
	dueInMs?: number;
	criticalPath: boolean;
};

export type TaskTriage = {
	items: TaskTriageItem[];
	openCount: number;
	blockedCount: number;
	overdueCount: number;
	dueSoonCount: number;
	criticalPathComplete: boolean;
	criticalPathMinutes?: number;
	criticalPathTaskIds: string[];
	criticalPathNote: string;
};

const dueSoonWindowMs = 4 * 60 * 60 * 1000;

function readableDuration(milliseconds: number): string {
	const totalMinutes = Math.max(1, Math.ceil(Math.abs(milliseconds) / 60_000));
	if (totalMinutes < 60) { return `${totalMinutes}m`; }
	const hours = Math.floor(totalMinutes / 60);
	const minutes = totalMinutes % 60;
	if (hours < 24) { return minutes ? `${hours}h ${minutes}m` : `${hours}h`; }
	const days = Math.floor(hours / 24);
	const remainingHours = hours % 24;
	return remainingHours ? `${days}d ${remainingHours}h` : `${days}d`;
}

function downstreamCounts(tasks: TaskTriageSource[]): Map<string, number> {
	const dependents = new Map<string, string[]>();
	for (const task of tasks) {
		if (task.status === 'done') { continue; }
		for (const dependency of task.dependencies || []) {
			const items = dependents.get(dependency) || [];
			items.push(task.id);
			dependents.set(dependency, items);
		}
	}
	const counts = new Map<string, number>();
	for (const task of tasks) {
		const seen = new Set<string>();
		const pending = [...(dependents.get(task.id) || [])];
		while (pending.length) {
			const dependent = pending.pop()!;
			if (seen.has(dependent) || dependent === task.id) { continue; }
			seen.add(dependent);
			pending.push(...(dependents.get(dependent) || []));
		}
		counts.set(task.id, seen.size);
	}
	return counts;
}

/** Rank room tasks for a local, recommendation-only triage view. */
export function triageRoomTasks(tasks: TaskTriageSource[], now = Date.now()): TaskTriage {
	const byId = new Map(tasks.map(task => [task.id, task]));
	const counts = downstreamCounts(tasks);
	const criticalPath = analyzeTaskCriticalPath(tasks);
	const criticalPathIds = new Set(criticalPath.taskIds);
	const openTasks = tasks.filter(task => task.status !== 'done');
	const items = openTasks.map(task => {
		const blockers = (task.dependencies || []).flatMap(id => {
			const dependency = byId.get(id);
			return !dependency ? ['Unavailable dependency'] : dependency.status === 'done' ? [] : [dependency.title];
		});
		const deadline = task.dueAt ? Date.parse(task.dueAt) : Number.NaN;
		const dueInMs = Number.isFinite(deadline) ? deadline - now : undefined;
		const downstreamCount = counts.get(task.id) || 0;
		const deadlineReason = dueInMs === undefined ? ''
			: dueInMs < 0 ? `Overdue by ${readableDuration(dueInMs)}`
				: `Due in ${readableDuration(dueInMs)}`;
		const baseRecommendation = blockers.length
			? `Blocked by ${blockers.join(', ')}${deadlineReason ? ` · ${deadlineReason.toLowerCase()}` : ''}`
			: deadlineReason || (downstreamCount ? `Unblocks ${downstreamCount} active task${downstreamCount === 1 ? '' : 's'}` : task.status === 'inProgress' ? 'Continue in-progress work' : 'Ready to start');
		const pathRecommendation = criticalPathIds.has(task.id) ? `Critical path · ${formatTaskEstimate(task.estimateMinutes)} estimate · ${baseRecommendation}` : baseRecommendation;
		const recommendation = task.priority === 'critical' || task.priority === 'high' ? `${taskPriorityLabel(task.priority)} priority · ${pathRecommendation}` : pathRecommendation;
		return { task, recommendation, blockers, downstreamCount, criticalPath: criticalPathIds.has(task.id), ...(dueInMs === undefined ? {} : { dueInMs }) };
	});
	items.sort((left, right) => {
		const blocked = Number(left.blockers.length > 0) - Number(right.blockers.length > 0);
		if (blocked) { return blocked; }
		const deadlineRank = (item: TaskTriageItem) => item.dueInMs === undefined ? 3 : item.dueInMs < 0 ? 0 : item.dueInMs <= dueSoonWindowMs ? 1 : 2;
		const urgency = deadlineRank(left) - deadlineRank(right);
		if (urgency) { return urgency; }
		const pathRank = Number(right.criticalPath) - Number(left.criticalPath);
		if (pathRank) { return pathRank; }
		const priority = taskPriorityRank(right.task.priority) - taskPriorityRank(left.task.priority);
		if (priority) { return priority; }
		const progress = Number(right.task.status === 'inProgress') - Number(left.task.status === 'inProgress');
		if (progress) { return progress; }
		const downstream = right.downstreamCount - left.downstreamCount;
		if (downstream) { return downstream; }
		const due = (left.dueInMs ?? Number.POSITIVE_INFINITY) - (right.dueInMs ?? Number.POSITIVE_INFINITY);
		return due || left.task.title.localeCompare(right.task.title);
	});
	return {
		items,
		openCount: openTasks.length,
		blockedCount: items.filter(item => item.blockers.length > 0).length,
		overdueCount: items.filter(item => item.dueInMs !== undefined && item.dueInMs < 0).length,
		dueSoonCount: items.filter(item => item.dueInMs !== undefined && item.dueInMs >= 0 && item.dueInMs <= dueSoonWindowMs).length,
		criticalPathComplete: criticalPath.complete,
		...(criticalPath.estimatedMinutes === undefined ? {} : { criticalPathMinutes: criticalPath.estimatedMinutes }),
		criticalPathTaskIds: criticalPath.taskIds,
		criticalPathNote: criticalPath.complete ? criticalPath.taskIds.length ? `Longest estimated unfinished chain is ${formatTaskEstimate(criticalPath.estimatedMinutes)}.` : 'No unfinished tasks are on the critical path.' : criticalPath.reason || 'Critical path unknown.'
	};
}
