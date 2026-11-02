import { taskPriorityLabel } from './taskPriority';
import { formatCountdownRemaining } from './countdownAlerts';
import { formatTaskEstimate } from './taskEstimate';
import { analyzeTaskCriticalPath } from './taskCriticalPath';
import { milestoneStatus, type RoomMilestone } from './roomMilestones';

export type DemoReadinessTask = {
	id: string;
	title: string;
	owner: string;
	status: 'open' | 'inProgress' | 'done';
	priority?: 'low' | 'normal' | 'high' | 'critical';
	estimateMinutes?: number;
	dependencies?: string[];
	acceptanceCriteria?: string[];
	completedCriteria?: string[];
	dueAt?: string;
};

export type DemoReadinessEvent = {
	id: string;
	at: string;
	kind: string;
	detail: string;
	taskId?: string;
};

export type DemoReadinessCountdown = { deadline?: string; pausedRemainingMs?: number; milestones?: readonly RoomMilestone[] };

export type ReadinessItem = { label: string; status: 'ready' | 'attention' | 'unknown'; detail: string };

function safeText(value: string, max = 300): string {
	return value.replace(/[\u0000-\u001f\u007f]/g, ' ').replace(/\s+/g, ' ').trim().slice(0, max).replace(/([\\`*_{}\[\]()#+\-.!|>])/g, '\\$1');
}

/** Build a conservative local readiness summary. Unknown external gates stay unknown. */
export function buildDemoReadiness(tasks: readonly DemoReadinessTask[], events: readonly DemoReadinessEvent[], now = Date.now(), countdown: DemoReadinessCountdown = {}): ReadinessItem[] {
	const byId = new Map(tasks.map(task => [task.id, task]));
	const open = tasks.filter(task => task.status !== 'done');
	const blocked = open.filter(task => (task.dependencies || []).some(id => byId.get(id)?.status !== 'done'));
	const overdue = open.filter(task => typeof task.dueAt === 'string' && Number.isFinite(Date.parse(task.dueAt)) && Date.parse(task.dueAt) < now);
	const incompleteCriteria = tasks.filter(task => task.status === 'done' && task.acceptanceCriteria?.some(criterion => !(task.completedCriteria || []).includes(criterion)));
	const recentVerification = events.filter(event => event.kind === 'test').slice(-20).reverse();
	const manualVerification = recentVerification.filter(event => /^manually reported\b/i.test(event.detail));
	const verification = recentVerification.filter(event => !/^manually reported\b/i.test(event.detail));
	const latestVerificationByTask = new Map<string, DemoReadinessEvent>();
	for (const event of verification) { if (event.taskId && !latestVerificationByTask.has(event.taskId)) { latestVerificationByTask.set(event.taskId, event); } }
	const latestVerification = verification[0];
	const failedVerification = verification.filter(event => /failed \(exit (?!0\d*\))/i.test(event.detail));
	const latestManualVerification = manualVerification[0];
	const hasUnresolvedManualReport = !!latestManualVerification && (!latestVerification || Date.parse(latestManualVerification.at) > Date.parse(latestVerification.at));
	const latestVerificationFailed = failedVerification.some(event => event.taskId ? latestVerificationByTask.get(event.taskId)?.id === event.id : latestVerification.id === event.id);
	const testStatus: ReadinessItem['status'] = !latestVerification ? 'unknown' : latestVerificationFailed ? 'attention' : hasUnresolvedManualReport ? 'unknown' : 'ready';
	const testDetail = !latestVerification
		? manualVerification.length
			? `No IDE-recorded verification result is available. ${manualVerification.length} participant-reported result(s) were recorded but are not independently verified: ${safeText(manualVerification[0].detail)}.`
			: 'No room verification result recorded; run the relevant tests and link evidence.'
		: hasUnresolvedManualReport
			? `Latest IDE-recorded result: ${safeText(latestVerification.detail)}. A newer participant-reported result is not verified; run the task in the IDE to refresh this gate.`
			: `${verification.length} recent IDE-recorded verification result(s); latest: ${safeText(latestVerification.detail)}. Participant-reported results are excluded${manualVerification.length ? ` (${manualVerification.length} recent)` : ''}.`;
	const taskStatus: ReadinessItem['status'] = !tasks.length ? 'unknown' : open.length ? 'attention' : 'ready';
	const pausedCountdown = Number.isSafeInteger(countdown.pausedRemainingMs) && (countdown.pausedRemainingMs as number) >= 0;
	const deadline = typeof countdown.deadline === 'string' ? Date.parse(countdown.deadline) : Number.NaN;
	const countdownRemaining = pausedCountdown ? countdown.pausedRemainingMs as number : Number.isFinite(deadline) ? deadline - now : undefined;
	const countdownStatus: ReadinessItem['status'] = countdownRemaining === undefined ? 'unknown' : countdownRemaining <= 60 * 60_000 ? 'attention' : 'ready';
	const countdownDetail = countdownRemaining === undefined
		? 'No shared room countdown is set.'
		: pausedCountdown
			? `Countdown is paused with ${formatCountdownRemaining(countdownRemaining)}.`
			: countdownRemaining <= 0
				? 'Shared countdown deadline has passed.'
				: `Shared countdown has ${formatCountdownRemaining(countdownRemaining)}.`;
	const criticalPath = analyzeTaskCriticalPath(tasks);
	const criticalPathStatus: ReadinessItem['status'] = !criticalPath.complete ? 'unknown'
		: countdownRemaining === undefined || pausedCountdown ? 'unknown'
			: criticalPath.estimatedMinutes! > Math.floor(Math.max(0, countdownRemaining) / 60_000) ? 'attention' : 'ready';
	const criticalPathDetail = !criticalPath.complete
		? criticalPath.reason || 'Critical path estimate is incomplete.'
		: !tasks.some(task => task.status !== 'done') ? 'No unfinished task chain remains.'
			: countdownRemaining === undefined ? 'Set a shared countdown to compare the estimated task chain with remaining time.'
				: pausedCountdown ? `Estimated unfinished chain is ${formatTaskEstimate(criticalPath.estimatedMinutes)}; countdown is paused, so deadline feasibility is unknown.`
					: criticalPathStatus === 'attention' ? `Estimated unfinished chain is ${formatTaskEstimate(criticalPath.estimatedMinutes)}, longer than the ${formatCountdownRemaining(Math.max(0, countdownRemaining))} remaining.`
						: `Estimated unfinished chain is ${formatTaskEstimate(criticalPath.estimatedMinutes)}; ${formatCountdownRemaining(countdownRemaining)} remain.`;
	const milestones = (countdown.milestones || []).filter(item => Number.isFinite(Date.parse(item.dueAt))).slice(0, 20);
	const remainingTasks = tasks.filter(task => task.status !== 'done');
	const remainingPath = analyzeTaskCriticalPath(remainingTasks);
	const milestoneStates = milestones.map(item => {
		const state = milestoneStatus(item, now);
		const availableMinutes = Math.floor((Date.parse(item.dueAt) - now) / 60_000);
		const atRisk = state === 'upcoming' && remainingPath.complete && remainingPath.estimatedMinutes! > availableMinutes;
		return { item, state, atRisk, remainingPath };
	});
	const milestoneWarnings = milestoneStates.filter(entry => entry.state !== 'upcoming' || entry.atRisk);
	const milestoneReadinessStatus: ReadinessItem['status'] = !milestones.length ? 'unknown' : milestoneWarnings.length ? 'attention' : milestoneStates.some(entry => !entry.remainingPath.complete) ? 'unknown' : 'ready';
	const describeMilestone = ({ item, state, atRisk, remainingPath }: typeof milestoneStates[number]) => `${safeText(item.label, 80)} (${state === 'passed' ? 'passed' : state === 'soon' ? `due in ${formatCountdownRemaining(Date.parse(item.dueAt) - now).replace(' remaining', '')}` : atRisk ? 'estimated work exceeds available time' : remainingPath.complete ? remainingPath.estimatedMinutes === 0 ? 'no unfinished work remains' : `estimated work fits; ${formatCountdownRemaining(Date.parse(item.dueAt) - now)} available` : 'estimate unknown'})`;
	const milestoneReadinessDetail = !milestones.length ? 'No shared room milestones are set.' : milestoneWarnings.length
		? `${milestoneWarnings.length} milestone(s) need attention: ${milestoneWarnings.slice(0, 8).map(describeMilestone).join('; ')}.`
		: `${milestones.length} upcoming milestone(s): ${milestoneStates.slice(0, 8).map(describeMilestone).join('; ')}${milestones.length > 8 ? '; more omitted' : ''}.`;
	return [
		{ label: 'Room tasks', status: taskStatus, detail: !tasks.length ? 'No room tasks are recorded; readiness cannot be inferred.' : open.length ? `${open.length} unfinished task(s): ${open.slice(0, 8).map(task => `${safeText(task.title, 100)} (${taskPriorityLabel(task.priority)} priority, ${task.status}, ${safeText(task.owner, 60) || 'unassigned'}, ${formatTaskEstimate(task.estimateMinutes)} estimate)`).join('; ')}${open.length > 8 ? '; more omitted' : ''}.` : `All ${tasks.length} room task(s) are marked done by their owners.` },
		{ label: 'Task dependencies', status: blocked.length ? 'attention' : 'ready', detail: blocked.length ? `${blocked.length} unfinished task(s) have incomplete or unavailable dependencies: ${blocked.slice(0, 8).map(task => safeText(task.title, 100)).join('; ')}.` : 'No unfinished task has an incomplete dependency.' },
		{ label: 'Task deadlines', status: overdue.length ? 'attention' : 'ready', detail: overdue.length ? `${overdue.length} unfinished task(s) are overdue: ${overdue.slice(0, 8).map(task => safeText(task.title, 100)).join('; ')}.` : 'No unfinished task with a valid deadline is overdue.' },
		{ label: 'Acceptance criteria', status: incompleteCriteria.length ? 'attention' : tasks.some(task => task.status === 'done' && task.acceptanceCriteria?.length) ? 'ready' : 'unknown', detail: incompleteCriteria.length ? `${incompleteCriteria.length} completed task(s) have unconfirmed acceptance criteria.` : 'Completed task criteria are owner attestations, not independent verification; tasks without criteria are not covered.' },
		{ label: 'Recorded verification', status: testStatus, detail: testDetail },
		{ label: 'Preview availability', status: 'unknown', detail: 'No preview health integration is configured; check the demo URL manually.' },
		{ label: 'Deployment health', status: 'unknown', detail: 'No deployment health integration is configured; verify the target manually.' },
		{ label: 'Submission links and materials', status: 'unknown', detail: 'Confirm the submission form, required links, demo script, and presentation materials manually.' },
		{ label: 'Countdown timing', status: countdownStatus, detail: countdownDetail },
		{ label: 'Estimated critical path', status: criticalPathStatus, detail: criticalPathDetail },
		{ label: 'Room milestones', status: milestoneReadinessStatus, detail: milestoneReadinessDetail }
	];
}

export function formatDemoReadinessMarkdown(project: string, generatedAt: string, items: readonly ReadinessItem[]): string {
	const status = (value: ReadinessItem['status']) => value === 'ready' ? 'Recorded ready' : value === 'attention' ? 'Needs attention' : 'Unknown / manual check';
	return `# Forge Room Demo Readiness\n\n- **Project:** ${safeText(project, 160) || 'Untitled workspace'}\n- **Generated:** ${generatedAt}\n- **Scope:** Local snapshot of room tasks and recent verification events; external systems are not queried.\n\n${items.map(item => `## ${safeText(item.label, 80)} — ${status(item.status)}\n\n${safeText(item.detail, 1200)}\n`).join('\n')}`;
}
