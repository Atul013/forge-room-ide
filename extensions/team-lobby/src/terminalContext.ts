import { redactVerificationCommand } from './verificationActivity';

export type TerminalContextEntry = { root?: string };

export function terminalRootAfterExecution(startRoot: string | undefined, endRoot: string | undefined): string | undefined {
	return startRoot && startRoot === endRoot ? startRoot : undefined;
}

export function terminalEntriesForWorkspace<T extends TerminalContextEntry>(entries: readonly T[], workspaceRoot: string, workspaceCount: number): T[] {
	return entries.filter(entry => entry.root ? entry.root === workspaceRoot : workspaceCount <= 1);
}

/** Return the newest captured command with output, never borrowing it from another project. */
export function latestTerminalEntryForWorkspace<T extends TerminalContextEntry & { output?: string }>(entries: readonly T[], workspaceRoot: string, workspaceCount: number): T | undefined {
	return terminalEntriesForWorkspace(entries, workspaceRoot, workspaceCount).filter(entry => typeof entry.output === 'string' && entry.output.length > 0).at(-1);
}

export function formatTerminalCaptureForModel(entry: { terminal: string; command: string; output: string; completed: boolean; exitCode?: number; truncated: boolean }): string {
	const terminal = entry.terminal.replace(/[\u0000-\u001f\u007f]/g, ' ').trim().slice(0, 120) || 'Terminal';
	const command = redactVerificationCommand(entry.command).replace(/[\r\n\u0000-\u001f\u007f]/g, ' ').trim().slice(0, 220) || '(command unavailable)';
	const status = entry.completed ? entry.exitCode === undefined ? 'finished; exit status unknown' : `finished with exit code ${entry.exitCode}` : 'still running; output may be partial';
	const output = entry.output.replace(/\r\n/g, '\n').replace(/\r/g, '\n').replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/g, '');
	return `Captured terminal output (${status}; ${output.length} characters${entry.truncated ? '; capture was truncated' : ''})\nTERMINAL: ${terminal}\nCOMMAND: ${command}\n\n${output}`;
}
