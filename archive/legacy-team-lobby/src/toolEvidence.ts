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
	if (tool === 'read_workspace_file') {
		const match = /^FILE ([^\r\n]+)/.exec(result);
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
				: /paused or canceled|request was canceled/i.test(result) ? 'canceled'
					: /tool call limit reached|tools were not enabled/i.test(result) ? 'rejected'
						: /^Could not (?:read|run)/i.test(result) ? 'failed'
						: /^No matching lines found/i.test(result) ? 'no_matches'
							: 'success';
	return {
		tool: tool === 'read_workspace_file' || tool === 'search_workspace' ? tool : 'unsupported_tool',
		sourcePaths: [...sourcePaths],
		outputCharacters: Math.min(30_000, result.length),
		outcome,
		completedAt
	};
}

export function normalizeWorkspaceToolEvidence(value: unknown): WorkspaceToolEvidence[] {
	if (!Array.isArray(value)) { return []; }
	return value.filter((item): item is WorkspaceToolEvidence => !!item && typeof item === 'object'
		&& ['read_workspace_file', 'search_workspace', 'unsupported_tool'].includes((item as WorkspaceToolEvidence).tool)
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
