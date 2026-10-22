const test = require('node:test');
const assert = require('node:assert/strict');
const { agentCapabilityAudit, agentCapabilityScopeMatches, boundCapabilityOutput, formatAgentCapabilityAudit, prepareAgentCapability } = require('../dist/agentCapabilities');

test('registered workspace tools normalize into distinct scoped capabilities', () => {
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
});

test('capability registry rejects unknown tools, extra fields, malformed inputs, and workspace escapes', () => {
	const invalid = [
		prepareAgentCapability('execute_shell', { command: 'rm -rf .' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('search_workspace', { query: 'x', glob: '../**', admin: true }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('read_workspace_file', { path: '../secret.txt' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('read_workspace_file', { path: 'C:\\secret.txt' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('read_workspace_file', { path: 'safe.txt', extra: 'ignored?' }, 'agent-1', 'Forge', 'file:///workspace'),
		prepareAgentCapability('search_workspace', { query: 'x\nforged log' }, 'agent-1', 'Forge', 'file:///workspace')
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
});
