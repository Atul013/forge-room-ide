import { verificationActivityDetail } from './verificationActivity';

export type ManualVerificationOutcome = 'passed' | 'failed' | 'stopped';

/** Format participant-reported evidence without implying Forge ran or verified it. */
export function manualVerificationActivityDetail(taskTitle: string | undefined, checkName: string, outcome: ManualVerificationOutcome): string | undefined {
	const label = checkName.trim();
	if (!label || label.length > 80 || /[\u0000-\u001f\u007f]/.test(label) || /(?:secret|token|password|passwd|api[_ -]?key|private[_ -]?key|credential|authorization)/i.test(label)) { return undefined; }
	const task = (taskTitle || 'unlinked room check').replace(/[\u0000-\u001f\u007f]/g, ' ').replace(/\s+/g, ' ').trim().slice(0, 100) || 'unlinked room check';
	const exitCode = outcome === 'passed' ? 0 : outcome === 'failed' ? 1 : undefined;
	return `manually reported ${verificationActivityDetail(task, label, exitCode)}`;
}
