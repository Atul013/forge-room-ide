const maxCommandLength = 220;

export type VerificationCommandEvidence = { command?: string; sharedWithRoom?: true };

export function canShareVerificationCommand(command: string): boolean {
	return command.trim().length > 0 && command.length <= maxCommandLength && !/[\r\n]/.test(command);
}

export function redactVerificationCommand(command: string): string {
	return command
		.replace(/\b(Bearer|Basic)(\s+)([^\s"'`]+)(["']?)/gi, '$1$2[REDACTED]$4')
		.replace(/(-u\s+|--user(?:=|\s+))([^:\s]+):([^\s"'`]+)(["']?)/gi, '$1[REDACTED]$4')
		.replace(/([a-z][a-z0-9+.-]*:\/\/)[^/@\s]+:[^/@\s]+@/gi, '$1[REDACTED]@')
		.replace(/(--?(?:api[-_]?key|access[-_]?key|token|password|passwd|secret|client[-_]?secret|private[-_]?key)(?:=|\s+))([^\s"'`]+)(["']?)/gi, '$1[REDACTED]$3')
		.replace(/\b([A-Z0-9_]*(?:API_KEY|ACCESS_KEY|TOKEN|PASSWORD|PASSWD|SECRET)[A-Z0-9_]*=)([^\s"'`]+)(["']?)/gi, '$1[REDACTED]$3')
		.replace(/\b(?:AKIA[A-Z0-9]{16}|github_pat_[A-Za-z0-9_]{20,}|gh[pousr]_[A-Za-z0-9_]{20,}|glpat-[A-Za-z0-9_-]{20,}|xox[baprs]-[A-Za-z0-9-]{10,}|sk-[A-Za-z0-9_-]{16,})\b/g, '[REDACTED_TOKEN]')
		.replace(/\beyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b/g, '[REDACTED_JWT]');
}

export function verificationCommandEvidence(command: string | undefined, shareWithRoom: boolean): VerificationCommandEvidence {
	if (!command || !canShareVerificationCommand(command)) { return {}; }
	const sanitized = redactVerificationCommand(command);
	return { command: sanitized, ...(shareWithRoom ? { sharedWithRoom: true as const } : {}) };
}

export function commandForRoom(evidence: VerificationCommandEvidence) {
	return evidence.sharedWithRoom ? evidence.command : undefined;
}

export function verificationActivityDetail(taskTitle: string, verificationName: string, exitCode?: number, command?: string): string {
	const outcome = exitCode === 0 ? 'passed (exit 0)' : exitCode === undefined ? 'completed without a process exit code' : `failed (exit ${exitCode})`;
	const commandText = command && canShareVerificationCommand(command) ? ` Command: ${redactVerificationCommand(command)}` : '';
	return `verification for task “${taskTitle.slice(0, 100)}”: ${verificationName.slice(0, 80)} ${outcome}.${commandText}`;
}
