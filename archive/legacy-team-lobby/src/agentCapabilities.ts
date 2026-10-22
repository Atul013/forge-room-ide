export type AgentCapabilityId = 'workspace.search' | 'workspace.readFile';
export type AgentCapabilityScope = 'workspace.read.search' | 'workspace.read.file';
export type AgentCapabilityInput = { query: string; glob: string } | { path: string };

export type AgentCapabilityRequest = {
	id: AgentCapabilityId;
	toolName: 'search_workspace' | 'read_workspace_file';
	actorId: string;
	workspaceName: string;
	workspaceRoot: string;
	scope: AgentCapabilityScope;
	resource: string;
	input: AgentCapabilityInput;
};

export type AgentCapabilityAudit = {
	capabilityId: AgentCapabilityId;
	actorId: string;
	workspace: string;
	scope: AgentCapabilityScope;
	resource: string;
	decision: 'approved' | 'denied';
	outcome: 'success' | 'no_matches' | 'rejected' | 'denied' | 'failed' | 'canceled';
	outputCharacters: number;
	completedAt: string;
};

const maxInputKeys = 3;
const controlCharacters = /[\u0000-\u001f\u007f]/;

function cleanLabel(value: string, maxLength: number): string {
	return value.replace(/[\u0000-\u001f\u007f]/g, ' ').trim().slice(0, maxLength) || 'unknown';
}

function inputObject(value: unknown): Record<string, unknown> | undefined {
	if (!value || typeof value !== 'object' || Array.isArray(value)) { return undefined; }
	const prototype = Object.getPrototypeOf(value);
	return prototype === Object.prototype || prototype === null ? value as Record<string, unknown> : undefined;
}

function hasOnlyKeys(value: Record<string, unknown>, allowed: string[]): boolean {
	const keys = Object.keys(value);
	return keys.length <= maxInputKeys && keys.every(key => allowed.includes(key));
}

function safeRelativePath(value: string, maxLength: number): string | undefined {
	const normalized = value.trim().replace(/\\/g, '/');
	if (!normalized || normalized.length > maxLength || controlCharacters.test(normalized) || normalized.startsWith('/') || /^[a-zA-Z]:/.test(normalized) || normalized.split('/').some(part => !part || part === '.' || part === '..')) { return undefined; }
	return normalized;
}

/** Normalize an IDE model tool call into a registered, workspace-scoped Forge capability. */
export function prepareAgentCapability(toolName: string, rawInput: unknown, actorId: string, workspaceName: string, workspaceRoot: string): { request?: AgentCapabilityRequest; error?: string } {
	if (!actorId.trim() || actorId.length > 100 || controlCharacters.test(actorId) || !workspaceName.trim() || workspaceName.length > 200 || controlCharacters.test(workspaceName) || !workspaceRoot.trim() || workspaceRoot.length > 2048 || controlCharacters.test(workspaceRoot)) {
		return { error: 'Rejected: invalid actor or workspace scope.' };
	}
	const input = inputObject(rawInput);
	if (!input) { return { error: 'Rejected: capability input must be a plain object.' }; }
	if (toolName === 'search_workspace') {
		if (!hasOnlyKeys(input, ['query', 'glob']) || typeof input.query !== 'string' || input.query.trim().length < 1 || input.query.trim().length > 200 || controlCharacters.test(input.query)) { return { error: 'Rejected: workspace search requires a plain text query of 1 to 200 characters.' }; }
		const glob = input.glob === undefined ? '**/*' : typeof input.glob === 'string' ? input.glob.trim() : '';
		if (!glob || glob.length > 120 || controlCharacters.test(glob) || glob.startsWith('/') || /^[a-zA-Z]:/.test(glob) || glob.split(/[\\/]/).includes('..')) { return { error: 'Rejected: workspace search glob must be a relative pattern of up to 120 characters.' }; }
		const request: AgentCapabilityRequest = { id: 'workspace.search', toolName, actorId: cleanLabel(actorId, 100), workspaceName: cleanLabel(workspaceName, 200), workspaceRoot, scope: 'workspace.read.search', resource: 'workspace', input: { query: input.query.trim(), glob } };
		return { request };
	}
	if (toolName === 'read_workspace_file') {
		if (!hasOnlyKeys(input, ['path']) || typeof input.path !== 'string') { return { error: 'Rejected: workspace file read requires only a relative path.' }; }
		const filePath = safeRelativePath(input.path, 500);
		if (!filePath) { return { error: 'Rejected: provide a safe workspace-relative file path.' }; }
		const request: AgentCapabilityRequest = { id: 'workspace.readFile', toolName, actorId: cleanLabel(actorId, 100), workspaceName: cleanLabel(workspaceName, 200), workspaceRoot, scope: 'workspace.read.file', resource: filePath, input: { path: filePath } };
		return { request };
	}
	return { error: 'Rejected: the requested capability is not registered.' };
}

/** Ensure the capability grant is being used against the workspace that requested it. */
export function agentCapabilityScopeMatches(request: AgentCapabilityRequest, workspaceRoot: string): boolean {
	return request.workspaceRoot === workspaceRoot;
}

/** Bound provider-returned text to the remaining per-request result budget. */
export function boundCapabilityOutput(value: string, remainingCharacters: number): { text: string; truncated: boolean } {
	const limit = Number.isFinite(remainingCharacters) ? Math.max(0, Math.floor(remainingCharacters)) : 0;
	return { text: value.slice(0, limit), truncated: value.length > limit };
}

/** Create a concise approval summary from validated capability input. */
export function agentCapabilityApprovalSummary(request: AgentCapabilityRequest): string {
	if (request.id === 'workspace.search') {
		const input = request.input as { query: string; glob: string };
		return `Search workspace for ${JSON.stringify(input.query)} in ${input.glob}`.slice(0, 500);
	}
	const input = request.input as { path: string };
	return `Read workspace file ${input.path}`.slice(0, 500);
}

/** Produce local audit metadata without retaining query strings, prompts, or result text. */
export function agentCapabilityAudit(request: AgentCapabilityRequest, decision: AgentCapabilityAudit['decision'], outcome: AgentCapabilityAudit['outcome'], outputCharacters: number, completedAt = new Date().toISOString()): AgentCapabilityAudit {
	return {
		capabilityId: request.id,
		actorId: request.actorId,
		workspace: request.workspaceName,
		scope: request.scope,
		resource: request.resource,
		decision,
		outcome,
		outputCharacters: Math.min(30_000, Math.max(0, Math.floor(outputCharacters))),
		completedAt: completedAt.slice(0, 40)
	};
}

/** Format bounded, content-free local audit metadata for the Forge Room output channel. */
export function formatAgentCapabilityAudit(audit: AgentCapabilityAudit): string {
	return `Capability audit ${JSON.stringify(audit)}`;
}

