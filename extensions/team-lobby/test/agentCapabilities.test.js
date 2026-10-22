const test = require('node:test');
const assert = require('node:assert/strict');
const { agentCapabilityAudit, agentCapabilityScopeMatches, boundCapabilityOutput, formatAgentCapabilityAudit, prepareAgentCapability, requiresAgentCapabilityApproval } = require('../dist/agentCapabilities');

test('registered workspace tools normalize into distinct scoped capabilities', () => {
	const openFile = prepareAgentCapability('editor_open', { path: 'src/main.ts', line: 12 }, 'agent-1', 'Forge', 'file:///workspace');
	assert.equal(openFile.request.id, 'editor.open');
	assert.equal(openFile.request.scope, 'editor.navigate');
	assert.deepEqual(openFile.request.input, { path: 'src/main.ts', line: 12 });
	assert.equal(requiresAgentCapabilityApproval(openFile.request.id, true), true);
	assert.match(require('../dist/agentCapabilities').agentCapabilityApprovalSummary(openFile.request), /changes the visible editor/);
	assert.equal(prepareAgentCapability('editor_open', { path: '.env' }, 'agent-1', 'Forge', 'file:///workspace').request, undefined);
	const inventory = prepareAgentCapability('workspace_list', {}, 'agent-1', 'Forge', 'file:///workspace');
	assert.equal(inventory.request.id, 'workspace.list');
	assert.equal(inventory.request.scope, 'workspace.read.list');
	assert.equal(inventory.request.resource, 'workspace');
	assert.match(require('../dist/agentCapabilities').agentCapabilityApprovalSummary(inventory.request), /No file contents are read/);
	assert.equal(prepareAgentCapability('workspace_list', { path: 'src' }, 'agent-1', 'Forge', 'file:///workspace').request, undefined);
	const search = prepareAgentCapability('search_workspace', { query: ' TODO ', glob: 'src/**/*.ts' }, 'agent-1', 'Forge', 'file:///workspace');
	assert.deepEqual(search.request, {
		id: 'workspace.search', toolName: 'search_workspace', actorId: 'agent-1', workspaceName: 'Forge', workspaceRoot: 'file:///workspace', scope: 'workspace.read.search', resource: 'workspace', input: { query: 'TODO', glob: 'src/**/*.ts' }
	});
	const read = prepareAgentCapability('read_workspace_file', { path: 'src\\main.ts' }, 'agent-1', 'Forge', 'file:///workspace');
	assert.equal(read.request.id, 'workspace.readFile');
	assert.equal(read.request.scope, 'workspace.read.file');
	assert.deepEqual(read.request.input, { path: 'src/main.ts' });
	assert.equal(agentCapabilityScopeMatches(read.request, 'file:///workspace'), true);
	assert.equal(agentCapabilityScopeMatches(read.request, 'file:///other-workspace'), false);
	const status = prepareAgentCapability('git_status', {}, 'agent-1', 'Forge', 'file:///workspace');
	assert.equal(status.request.id, 'workspace.gitStatus');
	assert.equal(status.request.scope, 'workspace.read.gitStatus');
	assert.equal(status.request.resource, 'workspace');
	assert.deepEqual(status.request.input, {});
	assert.equal(prepareAgentCapability('git_status', { path: 'outside' }, 'agent-1', 'Forge', 'file:///workspace').request, undefined);
	const diff = prepareAgentCapability('git_diff', { path: 'src/main.ts' }, 'agent-1', 'Forge', 'file:///workspace');
	assert.equal(diff.request.id, 'workspace.gitDiff');
	assert.equal(diff.request.scope, 'workspace.read.gitDiff');
	assert.equal(diff.request.resource, 'src/main.ts');
	assert.match(require('../dist/agentCapabilities').agentCapabilityApprovalSummary(diff.request), /patch content will be sent to the selected provider/);
	const workspaceDiagnostics = prepareAgentCapability('diagnostics_list', {}, 'agent-1', 'Forge', 'file:///workspace');
	assert.equal(workspaceDiagnostics.request.id, 'workspace.diagnostics');
	assert.equal(workspaceDiagnostics.request.resource, 'workspace');
	const fileDiagnostics = prepareAgentCapability('diagnostics_list', { path: 'src/main.ts' }, 'agent-1', 'Forge', 'file:///workspace');
	assert.equal(fileDiagnostics.request.resource, 'src/main.ts');
	assert.match(require('../dist/agentCapabilities').agentCapabilityApprovalSummary(fileDiagnostics.request), /diagnostic messages will be sent to the selected provider/);
	const symbols = prepareAgentCapability('symbols_find', { query: 'parseTask' }, 'agent-1', 'Forge', 'file:///workspace');
	assert.equal(symbols.request.id, 'workspace.symbols');
	assert.equal(symbols.request.scope, 'workspace.read.symbols');
	assert.equal(symbols.request.resource, 'workspace');
	assert.match(require('../dist/agentCapabilities').agentCapabilityApprovalSummary(symbols.request), /registered IDE symbol providers/);
	const testTasks = prepareAgentCapability('tests_discover', {}, 'agent-1', 'Forge', 'file:///workspace');
	assert.equal(testTasks.request.id, 'workspace.tests');
	assert.equal(testTasks.request.scope, 'workspace.read.tests');
	assert.match(require('../dist/agentCapabilities').agentCapabilityApprovalSummary(testTasks.request), /does not run a task/);
	assert.equal(prepareAgentCapability('tests_discover', { command: 'unsafe' }, 'agent-1', 'Forge', 'file:///workspace').request, undefined);
	const testRun = prepareAgentCapability('tests_run', { name: 'unit tests', source: 'npm' }, 'agent-1', 'Forge', 'file:///workspace');
	assert.equal(testRun.request.id, 'workspace.testRun');
	assert.equal(testRun.request.scope, 'workspace.execute.tests');
	assert.match(require('../dist/agentCapabilities').agentCapabilityApprovalSummary(testRun.request), /OS account privileges/);
	assert.equal(requiresAgentCapabilityApproval(testRun.request.id, true), true);
	assert.equal(requiresAgentCapabilityApproval(testTasks.request.id, true), false);
	assert.equal(requiresAgentCapabilityApproval(testTasks.request.id, false), true);
	assert.equal(prepareAgentCapability('tests_run', { name: 'unit tests', source: 'npm', command: 'evil' }, 'agent-1', 'Forge', 'file:///workspace').request, undefined);
	const terminalOutput = prepareAgentCapability('terminal_last_output', {}, 'agent-1', 'Forge', 'file:///workspace');
	assert.equal(terminalOutput.request.id, 'workspace.terminalOutput');
	assert.equal(terminalOutput.request.scope, 'workspace.read.terminalOutput');
	assert.match(require('../dist/agentCapabilities').agentCapabilityApprovalSummary(terminalOutput.request), /output may contain sensitive information/);
	assert.equal(requiresAgentCapabilityApproval(terminalOutput.request.id, true), true);
	assert.equal(agentCapabilityScopeMatches(terminalOutput.request, 'file:///workspace'), true);
	assert.equal(prepareAgentCapability('terminal_last_output', { command: 'npm test' }, 'agent-1', 'Forge', 'file:///workspace').request, undefined);
	const edit = prepareAgentCapability('apply_workspace_edit', { path: 'src/main.ts', oldText: 'const answer = 1;', newText: 'const answer = 2;' }, 'agent-1', 'Forge', 'file:///workspace');
	assert.equal(edit.request.id, 'workspace.applyEdit');
	assert.equal(edit.request.scope, 'workspace.write.patch');
	assert.equal(edit.request.resource, 'src/main.ts');
	assert.equal(requiresAgentCapabilityApproval(edit.request.id, true), true);
	assert.equal(agentCapabilityScopeMatches(edit.request, 'file:///workspace'), true);
	assert.match(require('../dist/agentCapabilities').agentCapabilityApprovalSummary(edit.request), /complete change will be previewed/);
	const debuggerFrames = prepareAgentCapability('debugger_frames', {}, 'agent-1', 'Forge', 'file:///workspace');
	assert.equal(debuggerFrames.request.id, 'debugger.frames');
	assert.equal(debuggerFrames.request.scope, 'debugger.read.frames');
	assert.equal(debuggerFrames.request.resource, 'active debug session');
	assert.equal(agentCapabilityScopeMatches(debuggerFrames.request, 'file:///workspace'), true);
	assert.equal(agentCapabilityScopeMatches(debuggerFrames.request, 'file:///other-workspace'), false);
	assert.match(require('../dist/agentCapabilities').agentCapabilityApprovalSummary(debuggerFrames.request), /No variables or debug actions are accessed/);
	assert.equal(prepareAgentCapability('debugger_frames', { threadId: 5 }, 'agent-1', 'Forge', 'file:///workspace').request, undefined);
});

test('capability registry rejects unknown tools, extra fields, malformed inputs, and workspace escapes', () => {
	const invalid = [
		prepareAgentCapability('editor_open', { path: '../outside.ts' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('editor_open', { path: 'src/main.ts', line: 0 }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('editor_open', { path: 'src/main.ts', line: 1.5 }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('editor_open', { path: 'src/main.ts', line: 1, extra: true }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('workspace_list', { include: '**/*' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('execute_shell', { command: 'rm -rf .' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('search_workspace', { query: 'x', glob: '../**', admin: true }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('read_workspace_file', { path: '../secret.txt' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('read_workspace_file', { path: 'C:\\secret.txt' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('read_workspace_file', { path: 'safe.txt', extra: 'ignored?' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('git_diff', { path: '../outside.ts' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('git_diff', { path: '.env' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('git_diff', { path: 'src/a.ts', extra: 'ignored?' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('diagnostics_list', { path: '../outside.ts' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('diagnostics_list', { path: '.env' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('diagnostics_list', { path: 'src/a.ts', extra: 'ignored?' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('symbols_find', { query: 'bad\nquery' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('symbols_find', { query: 'x'.repeat(161) }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('symbols_find', { query: 'x', extra: true }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('tests_discover', { extra: true }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('tests_run', { name: 'unit tests\nAllow task', source: 'npm' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('search_workspace', { query: 'x\nforged log' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('apply_workspace_edit', { path: '.env', oldText: 'A=1', newText: 'A=2' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('apply_workspace_edit', { path: '../outside.ts', oldText: 'a', newText: 'b' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('apply_workspace_edit', { path: 'src/a.ts', oldText: '', newText: 'b' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('apply_workspace_edit', { path: 'src/a.ts', oldText: 'a', newText: 'a' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('apply_workspace_edit', { path: 'src/a.ts', oldText: '\u0000', newText: 'b' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('apply_workspace_edit', { path: 'src/a.ts', oldText: 'a'.repeat(12_001), newText: 'b' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('apply_workspace_edit', { path: 'src/a.ts', oldText: 'a', newText: 'b', extra: true }, 'agent-1', 'Forge', 'file:///workspace')
	];
	assert.equal(invalid.every(result => !result.request && result.error?.startsWith('Rejected:')), true);
});

test('provider output is capped at the remaining request budget', () => {
	assert.deepEqual(boundCapabilityOutput('abcdef', 4), { text: 'abcd', truncated: true });
	assert.deepEqual(boundCapabilityOutput('abc', 3), { text: 'abc', truncated: false });
	assert.deepEqual(boundCapabilityOutput('abc', -1), { text: '', truncated: true });
});

test('capability audit captures actor, scope, resource, decision and outcome without query or contents', () => {
	const request = prepareAgentCapability('search_workspace', { query: 'private-query', glob: 'src/**' }, 'agent-1', 'Forge', 'file:///workspace').request;
	const audit = agentCapabilityAudit(request, 'approved', 'success', 23, '2026-10-21T00:00:00.000Z');
	const serialized = formatAgentCapabilityAudit(audit);
	assert.deepEqual(audit, { capabilityId: 'workspace.search', actorId: 'agent-1', workspace: 'Forge', scope: 'workspace.read.search', resource: 'workspace', decision: 'approved', outcome: 'success', outputCharacters: 23, completedAt: '2026-10-21T00:00:00.000Z' });
	assert.equal(serialized.includes('private-query'), false);
	assert.equal(serialized.includes('result contents'), false);
	const symbolsRequest = prepareAgentCapability('symbols_find', { query: 'privateSymbolQuery' }, 'agent-1', 'Forge', 'file:///workspace').request;
	assert.equal(formatAgentCapabilityAudit(agentCapabilityAudit(symbolsRequest, 'approved', 'success', 42)).includes('privateSymbolQuery'), false);
});
