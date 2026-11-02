import { isSensitiveContextTarget } from './contextFiles';

export type AgentCapabilityId = 'editor.open' | 'workspace.list' | 'workspace.search' | 'workspace.readFile' | 'workspace.gitStatus' | 'workspace.gitDiff' | 'workspace.diagnostics' | 'workspace.symbols' | 'workspace.tests' | 'workspace.testRun' | 'workspace.terminalOutput' | 'workspace.applyEdit' | 'debugger.frames';
export type AgentCapabilityScope = 'editor.navigate' | 'workspace.read.list' | 'workspace.read.search' | 'workspace.read.file' | 'workspace.read.gitStatus' | 'workspace.read.gitDiff' | 'workspace.read.diagnostics' | 'workspace.read.symbols' | 'workspace.read.tests' | 'workspace.execute.tests' | 'workspace.read.terminalOutput' | 'workspace.write.patch' | 'debugger.read.frames';
export type AgentCapabilityInput = { query: string; glob: string } | { query: string } | { path: string; line?: number } | { path: string } | { path?: string } | { name: string; source: string } | { path: string; oldText: string; newText: string } | Record<string, never>;

export type AgentCapabilityRequest = {
	id: AgentCapabilityId;
	toolName: 'editor_open' | 'workspace_list' | 'search_workspace' | 'read_workspace_file' | 'git_status' | 'git_diff' | 'diagnostics_list' | 'symbols_find' | 'tests_discover' | 'tests_run' | 'terminal_last_output' | 'apply_workspace_edit' | 'debugger_frames';
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
	if (toolName === 'editor_open') {
		if (!hasOnlyKeys(input, ['path', 'line']) || typeof input.path !== 'string' || (input.line !== undefined && (!Number.isInteger(input.line) || (input.line as number) < 1 || (input.line as number) > 1_000_000))) { return { error: 'Rejected: editor open requires a relative file path and an optional positive one-based line number.' }; }
		const filePath = safeRelativePath(input.path, 500);
		if (!filePath || isSensitiveContextTarget(filePath, filePath)) { return { error: 'Rejected: provide a safe, non-sensitive workspace-relative file path.' }; }
		const request: AgentCapabilityRequest = { id: 'editor.open', toolName, actorId: cleanLabel(actorId, 100), workspaceName: cleanLabel(workspaceName, 200), workspaceRoot, scope: 'editor.navigate', resource: filePath, input: { path: filePath, ...(input.line === undefined ? {} : { line: input.line as number }) } };
		return { request };
	}
	if (toolName === 'workspace_list') {
		if (!hasOnlyKeys(input, [])) { return { error: 'Rejected: workspace inventory takes no arguments.' }; }
		const request: AgentCapabilityRequest = { id: 'workspace.list', toolName, actorId: cleanLabel(actorId, 100), workspaceName: cleanLabel(workspaceName, 200), workspaceRoot, scope: 'workspace.read.list', resource: 'workspace', input: {} };
		return { request };
	}
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
	if (toolName === 'git_status') {
		if (!hasOnlyKeys(input, [])) { return { error: 'Rejected: Git status takes no arguments.' }; }
		const request: AgentCapabilityRequest = { id: 'workspace.gitStatus', toolName, actorId: cleanLabel(actorId, 100), workspaceName: cleanLabel(workspaceName, 200), workspaceRoot, scope: 'workspace.read.gitStatus', resource: 'workspace', input: {} };
		return { request };
	}
	if (toolName === 'git_diff') {
		if (!hasOnlyKeys(input, ['path']) || typeof input.path !== 'string') { return { error: 'Rejected: Git diff requires only a workspace-relative file path.' }; }
		const filePath = safeRelativePath(input.path, 500);
		if (!filePath) { return { error: 'Rejected: provide a safe workspace-relative file path.' }; }
		if (isSensitiveContextTarget(filePath, filePath)) { return { error: 'Rejected: likely sensitive paths are not available through Git diff.' }; }
		const request: AgentCapabilityRequest = { id: 'workspace.gitDiff', toolName, actorId: cleanLabel(actorId, 100), workspaceName: cleanLabel(workspaceName, 200), workspaceRoot, scope: 'workspace.read.gitDiff', resource: filePath, input: { path: filePath } };
		return { request };
	}
	if (toolName === 'diagnostics_list') {
		if (!hasOnlyKeys(input, ['path']) || (input.path !== undefined && typeof input.path !== 'string')) { return { error: 'Rejected: diagnostics accepts only an optional workspace-relative file path.' }; }
		const filePath = input.path === undefined ? undefined : safeRelativePath(input.path as string, 500);
		if (input.path !== undefined && !filePath) { return { error: 'Rejected: provide a safe workspace-relative diagnostic path.' }; }
		if (filePath && isSensitiveContextTarget(filePath, filePath)) { return { error: 'Rejected: diagnostics for likely sensitive paths are not available.' }; }
		const request: AgentCapabilityRequest = { id: 'workspace.diagnostics', toolName, actorId: cleanLabel(actorId, 100), workspaceName: cleanLabel(workspaceName, 200), workspaceRoot, scope: 'workspace.read.diagnostics', resource: filePath || 'workspace', input: filePath ? { path: filePath } : {} };
		return { request };
	}
	if (toolName === 'symbols_find') {
		if (!hasOnlyKeys(input, ['query']) || typeof input.query !== 'string' || input.query.trim().length < 1 || input.query.trim().length > 160 || controlCharacters.test(input.query)) { return { error: 'Rejected: symbol search requires a plain text query of 1 to 160 characters.' }; }
		const query = input.query.trim();
		const request: AgentCapabilityRequest = { id: 'workspace.symbols', toolName, actorId: cleanLabel(actorId, 100), workspaceName: cleanLabel(workspaceName, 200), workspaceRoot, scope: 'workspace.read.symbols', resource: 'workspace', input: { query } };
		return { request };
	}
	if (toolName === 'tests_discover') {
		if (!hasOnlyKeys(input, [])) { return { error: 'Rejected: test discovery takes no arguments.' }; }
		const request: AgentCapabilityRequest = { id: 'workspace.tests', toolName, actorId: cleanLabel(actorId, 100), workspaceName: cleanLabel(workspaceName, 200), workspaceRoot, scope: 'workspace.read.tests', resource: 'workspace', input: {} };
		return { request };
	}
	if (toolName === 'tests_run') {
		if (!hasOnlyKeys(input, ['name', 'source']) || typeof input.name !== 'string' || typeof input.source !== 'string' || !input.name.trim() || !input.source.trim() || input.name.length > 160 || input.source.length > 80 || controlCharacters.test(input.name) || controlCharacters.test(input.source)) { return { error: 'Rejected: running a test task requires its exact listed name and provider.' }; }
		const name = cleanLabel(input.name, 160);
		const source = cleanLabel(input.source, 80);
		const request: AgentCapabilityRequest = { id: 'workspace.testRun', toolName, actorId: cleanLabel(actorId, 100), workspaceName: cleanLabel(workspaceName, 200), workspaceRoot, scope: 'workspace.execute.tests', resource: 'selected test task', input: { name, source } };
		return { request };
	}
	if (toolName === 'debugger_frames') {
		if (!hasOnlyKeys(input, [])) { return { error: 'Rejected: debugger frame inspection takes no arguments.' }; }
		const request: AgentCapabilityRequest = { id: 'debugger.frames', toolName, actorId: cleanLabel(actorId, 100), workspaceName: cleanLabel(workspaceName, 200), workspaceRoot, scope: 'debugger.read.frames', resource: 'active debug session', input: {} };
		return { request };
	}
	if (toolName === 'terminal_last_output') {
		if (!hasOnlyKeys(input, [])) { return { error: 'Rejected: captured terminal output takes no arguments.' }; }
		const request: AgentCapabilityRequest = { id: 'workspace.terminalOutput', toolName, actorId: cleanLabel(actorId, 100), workspaceName: cleanLabel(workspaceName, 200), workspaceRoot, scope: 'workspace.read.terminalOutput', resource: 'captured terminal output', input: {} };
		return { request };
	}
	if (toolName === 'apply_workspace_edit') {
		if (!hasOnlyKeys(input, ['path', 'oldText', 'newText']) || typeof input.path !== 'string' || typeof input.oldText !== 'string' || typeof input.newText !== 'string') { return { error: 'Rejected: workspace edit requires only a relative path, exact old text, and replacement text.' }; }
		const filePath = safeRelativePath(input.path, 500);
		if (!filePath || isSensitiveContextTarget(filePath, filePath)) { return { error: 'Rejected: provide a safe, non-sensitive workspace-relative file path.' }; }
		if (!input.oldText.length || input.oldText.length > 12_000 || input.newText.length > 12_000 || input.oldText === input.newText || controlCharacters.test(input.path) || input.oldText.includes('\u0000') || input.newText.includes('\u0000')) { return { error: 'Rejected: workspace edit must replace 1 to 12,000 characters with different text and stay within the per-call size limit.' }; }
		const request: AgentCapabilityRequest = { id: 'workspace.applyEdit', toolName, actorId: cleanLabel(actorId, 100), workspaceName: cleanLabel(workspaceName, 200), workspaceRoot, scope: 'workspace.write.patch', resource: filePath, input: { path: filePath, oldText: input.oldText, newText: input.newText } };
		return { request };
	}
	return { error: 'Rejected: the requested capability is not registered.' };
}

/** Ensure the capability grant is being used against the workspace that requested it. */
export function agentCapabilityScopeMatches(request: AgentCapabilityRequest, workspaceRoot: string): boolean {
	return request.workspaceRoot === workspaceRoot;
}

/** A request-wide read-only grant must never authorize execution of project task code. */
export function requiresAgentCapabilityApproval(id: AgentCapabilityId, requestScopedReadOnlyGrant: boolean): boolean {
	return id === 'editor.open' || id === 'workspace.testRun' || id === 'workspace.terminalOutput' || id === 'workspace.applyEdit' || !requestScopedReadOnlyGrant;
}

/** Bound provider-returned text to the remaining per-request result budget. */
export function boundCapabilityOutput(value: string, remainingCharacters: number): { text: string; truncated: boolean } {
	const limit = Number.isFinite(remainingCharacters) ? Math.max(0, Math.floor(remainingCharacters)) : 0;
	return { text: value.slice(0, limit), truncated: value.length > limit };
}

/** Create a concise approval summary from validated capability input. */
export function agentCapabilityApprovalSummary(request: AgentCapabilityRequest): string {
	if (request.id === 'editor.open') { const input = request.input as { path: string; line?: number }; return `Open ${input.path}${input.line ? ` at line ${input.line}` : ''} in the selected workspace editor. This changes the visible editor but does not send file contents.`.slice(0, 500); }
	if (request.id === 'workspace.list') { return 'List up to 250 non-sensitive file paths in the selected workspace; file names will be sent to the selected provider. No file contents are read.'; }
	if (request.id === 'workspace.search') {
		const input = request.input as { query: string; glob: string };
		return `Search workspace for ${JSON.stringify(input.query)} in ${input.glob}`.slice(0, 500);
	}
	if (request.id === 'workspace.gitStatus') { return 'Read Git status for the selected workspace (branch, change counts, and non-sensitive file names)'; }
	if (request.id === 'workspace.gitDiff') { return `Read the tracked Git diff for ${request.resource}; patch content will be sent to the selected provider`.slice(0, 500); }
	if (request.id === 'workspace.diagnostics') { return `Read diagnostics for ${request.resource}; diagnostic messages will be sent to the selected provider`.slice(0, 500); }
	if (request.id === 'workspace.symbols') { const input = request.input as { query: string }; return `Find workspace symbols matching ${JSON.stringify(input.query)}; the query goes to registered IDE symbol providers, and names/locations go to the selected model`.slice(0, 500); }
	if (request.id === 'workspace.tests') { return 'List registered IDE test tasks for the selected workspace; task labels and provider names will be sent to the selected model. This does not run a task.'; }
	if (request.id === 'workspace.testRun') { const input = request.input as { name: string; source: string }; return `Run the IDE test task ${JSON.stringify(input.name)} from ${JSON.stringify(input.source)} after explicit confirmation. Project task code runs with your OS account privileges and may modify files or use the network; task output stays local and only the exit result is sent to the selected model.`.slice(0, 500); }
	if (request.id === 'debugger.frames') { return 'Read up to 20 stack frames from the active paused debug thread in this workspace; frame names and workspace-relative source locations will be sent to the selected provider. No variables or debug actions are accessed.'; }
	if (request.id === 'workspace.terminalOutput') { return 'Share captured terminal command output with the selected provider after exact per-call confirmation; output may contain sensitive information.'; }
	if (request.id === 'workspace.applyEdit') { return `Apply an exact text replacement in ${request.resource}; the complete change will be previewed and requires separate approval`.slice(0, 500); }
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

