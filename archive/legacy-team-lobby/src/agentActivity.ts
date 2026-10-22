export type AgentActivityStage = 'started' | 'proposal' | 'applied' | 'paused' | 'canceled' | 'noProposal' | 'incomplete' | 'failed';

export function validTaskLink(kind: string, taskId: unknown): boolean {
	return taskId === undefined || (['agent', 'test', 'review'].includes(kind) && typeof taskId === 'string' && taskId.trim().length > 0 && taskId.length <= 100 && !/[\r\n\0]/.test(taskId));
}

/** Build room-safe summaries. This deliberately accepts no prompt or context data. */
export function agentActivityDetail(stage: AgentActivityStage, taskTitle: string, fileCount = 0, checkpointId = '', changedPaths: string[] = []): string {
	let detail: string;
	switch (stage) {
		case 'started': detail = `Build started for room task: ${taskTitle}. Prompt and selected context remain private.`; break;
		case 'proposal': detail = `Build prepared a proposal affecting ${fileCount} file${fileCount === 1 ? '' : 's'} for room task: ${taskTitle}. It is ready for review; no changes are applied until approval.`; break;
		case 'applied': {
			const paths = changedPaths.slice(0, 4).map(path => path.replace(/[\r\n]/g, ' ').slice(0, 60));
			const pathSummary = paths.length ? ` Changed paths: ${paths.join(', ')}${changedPaths.length > paths.length ? ', …' : ''}.` : '';
			detail = `Build applied changes to ${fileCount} file${fileCount === 1 ? '' : 's'} for room task: ${taskTitle}.${pathSummary} Checkpoint ${checkpointId.slice(0, 8)} is available for review.`;
			break;
		}
		case 'paused': detail = `Build generation was paused for room task: ${taskTitle}. No proposed files were applied; the session can be resumed by its owner.`; break;
		case 'canceled': detail = `Build generation was canceled for room task: ${taskTitle}. No proposed files were applied.`; break;
		case 'noProposal': detail = `Build returned no proposal for room task: ${taskTitle}. No files were applied.`; break;
		case 'incomplete': detail = `Build returned an incomplete proposal for room task: ${taskTitle}. No files were applied.`; break;
		case 'failed': detail = `Build could not finish for room task: ${taskTitle}. No files were applied.`; break;
	}
	return detail.slice(0, 500);
}
