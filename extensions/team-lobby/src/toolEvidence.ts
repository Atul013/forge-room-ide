export type WorkspaceToolEvidence = {
	tool: string;
	sourcePaths: string[];
	outputCharacters: number;
	outcome: 'success' | 'no_matches' | 'rejected' | 'denied' | 'failed' | 'canceled';
	completedAt: string;
};

const pathFromSearchLine = /^(.+):\d+: /;

/** Keep source provenance and outcome metadata, never the workspace result text or search query. */
export function workspaceToolEvidence(tool: string, result: string, completedAt = new Date().toISOString()): WorkspaceToolEvidence {
	const sourcePaths = new Set<string>();
	if (tool === 'git_status') {
		for (const line of result.split(/\r?\n/)) {
			const match = /^\s{2}([^\r\n]+)\s+\((?:staged|modified|untracked|conflict)\)$/.exec(line);
			if (match?.[1]) { sourcePaths.add(match[1].slice(0, 500)); }
			if (sourcePaths.size >= 40) { break; }
		}
	} else if (tool === 'git_diff') {
		const match = /^Git diff for ([^\r\n]+)/.exec(result);
		if (match?.[1]) { sourcePaths.add(match[1].slice(0, 500)); }
	} else if (tool === 'diagnostics_list') {
		for (const line of result.split(/\r?\n/)) {
			const match = /^FILE ([^\r\n]+)/.exec(line);
			if (match?.[1]) { sourcePaths.add(match[1].slice(0, 500)); }
			if (sourcePaths.size >= 40) { break; }
		}
	} else if (tool === 'symbols_find') {
		for (const line of result.split(/\r?\n/)) {
			const match = /^FILE ([^\r\n]+)/.exec(line);
			if (match?.[1]) { sourcePaths.add(match[1].slice(0, 500)); }
			if (sourcePaths.size >= 40) { break; }
		}
	} else if (tool === 'read_workspace_file') {
		const match = /^FILE ([^\r\n]+)/.exec(result);
		if (match?.[1]) { sourcePaths.add(match[1].slice(0, 500)); }
	} else if (tool === 'debugger_frames') {
		for (const line of result.split(/\r?\n/)) {
			const match = /^FRAME [^\r\n]+ — (.+):\d+$/.exec(line);
			if (match?.[1]) { sourcePaths.add(match[1].slice(0, 500)); }
			if (sourcePaths.size >= 40) { break; }
		}
	} else if (tool === 'apply_workspace_edit') {
		const match = /^Applied workspace edit to ([^\r\n]+)$/m.exec(result);
		if (match?.[1]) { sourcePaths.add(match[1].slice(0, 500)); }
	} else if (tool === 'editor_open') {
		const match = /^Opened workspace file: (.+?)(?: at line \d+)?$/m.exec(result);
		if (match?.[1]) { sourcePaths.add(match[1].slice(0, 500)); }
	} else if (tool === 'search_workspace') {
		for (const line of result.split(/\r?\n/)) {
			const match = pathFromSearchLine.exec(line);
			if (match?.[1]) { sourcePaths.add(match[1].slice(0, 500)); }
			if (sourcePaths.size >= 40) { break; }
		}
	}
	const outcome: WorkspaceToolEvidence['outcome'] =
		/^Rejected:/i.test(result) ? 'rejected'
			: /user denied|already denied|user declined/i.test(result) ? 'denied'
				: /paused or canceled|request was canceled|execution was canceled|user canceled/i.test(result) ? 'canceled'
					: /tool call limit reached|tools were not enabled/i.test(result) ? 'rejected'
						: /no longer available/i.test(result) ? 'rejected'
				: /^Could not (?:read|run|start|prepare|apply)/i.test(result) || /exceeded the 10-minute limit|without a process exit status|unverified/i.test(result) ? 'failed'
							: /exit code (\d+)/i.test(result) ? (Number(/exit code (\d+)/i.exec(result)?.[1]) === 0 ? 'success' : 'failed')
			: /^(?:Workspace files \(0|No matching lines found|No tracked changes|No diagnostics found|No symbols found|No registered test tasks|No workspace stack frames|No captured terminal output)/i.test(result) ? 'no_matches'
							: 'success';
	return {
		tool: tool === 'editor_open' || tool === 'workspace_list' || tool === 'read_workspace_file' || tool === 'search_workspace' || tool === 'git_status' || tool === 'git_diff' || tool === 'diagnostics_list' || tool === 'symbols_find' || tool === 'tests_discover' || tool === 'tests_run' || tool === 'terminal_last_output' || tool === 'apply_workspace_edit' || tool === 'debugger_frames' ? tool : 'unsupported_tool',
		sourcePaths: [...sourcePaths],
		outputCharacters: Math.min(30_000, result.length),
		outcome,
		completedAt
	};
}

export function normalizeWorkspaceToolEvidence(value: unknown): WorkspaceToolEvidence[] {
	if (!Array.isArray(value)) { return []; }
	return value.filter((item): item is WorkspaceToolEvidence => !!item && typeof item === 'object'
		&& ['editor_open', 'workspace_list', 'read_workspace_file', 'search_workspace', 'git_status', 'git_diff', 'diagnostics_list', 'symbols_find', 'tests_discover', 'tests_run', 'terminal_last_output', 'apply_workspace_edit', 'debugger_frames', 'unsupported_tool'].includes((item as WorkspaceToolEvidence).tool)
		&& Array.isArray((item as WorkspaceToolEvidence).sourcePaths)
		&& (item as WorkspaceToolEvidence).sourcePaths.length <= 40
		&& (item as WorkspaceToolEvidence).sourcePaths.every(source => typeof source === 'string' && source.length <= 500 && !/[\r\n]/.test(source))
		&& Number.isInteger((item as WorkspaceToolEvidence).outputCharacters)
		&& (item as WorkspaceToolEvidence).outputCharacters >= 0
		&& (item as WorkspaceToolEvidence).outputCharacters <= 30_000
		&& ['success', 'no_matches', 'rejected', 'denied', 'failed', 'canceled'].includes((item as WorkspaceToolEvidence).outcome)
		&& typeof (item as WorkspaceToolEvidence).completedAt === 'string'
		&& (item as WorkspaceToolEvidence).completedAt.length <= 40
		&& !Number.isNaN(Date.parse((item as WorkspaceToolEvidence).completedAt)))
		.slice(-20);
}

export function workspaceToolProgress(evidence: WorkspaceToolEvidence): string {
	const outcome = {
		success: 'completed', no_matches: 'no matches', rejected: 'rejected', denied: 'declined', failed: 'failed', canceled: 'canceled'
	}[evidence.outcome];
	const paths = evidence.sourcePaths.slice(0, 3).join(', ');
	const more = evidence.sourcePaths.length > 3 ? `, and ${evidence.sourcePaths.length - 3} more` : '';
	return `${evidence.tool}: ${outcome}${paths ? ` · ${paths}${more}` : ''} · ${evidence.outputCharacters.toLocaleString()} characters returned`;
}
