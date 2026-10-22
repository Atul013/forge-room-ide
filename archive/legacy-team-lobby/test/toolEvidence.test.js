const test = require('node:test');
const assert = require('node:assert/strict');
const { normalizeWorkspaceToolEvidence, workspaceToolEvidence, workspaceToolProgress } = require('../dist/toolEvidence');

test('read evidence records a source and output size without retaining file content', () => {
	const evidence = workspaceToolEvidence('read_workspace_file', 'FILE src/main.ts\nprivate source text', '2026-01-01T00:00:00.000Z');
	assert.deepEqual(evidence, {
		tool: 'read_workspace_file',
		sourcePaths: ['src/main.ts'],
		outputCharacters: 36,
		outcome: 'success',
		completedAt: '2026-01-01T00:00:00.000Z'
	});
	assert.equal(JSON.stringify(evidence).includes('private source text'), false);
});

test('search evidence records matched paths but not its query or matching lines', () => {
	const evidence = workspaceToolEvidence('search_workspace', 'Search results for "secretQuery":\nsrc/one.ts:4: secretQuery=password\nsrc/two.ts:8: secretQuery=token');
	assert.deepEqual(evidence.sourcePaths, ['src/one.ts', 'src/two.ts']);
	assert.equal(JSON.stringify(evidence).includes('secretQuery'), false);
	assert.equal(JSON.stringify(evidence).includes('password'), false);
});

test('evidence labels denied, rejected, failed, canceled and empty-search outcomes', () => {
	assert.equal(workspaceToolEvidence('read_workspace_file', 'The user denied access').outcome, 'denied');
	assert.equal(workspaceToolEvidence('read_workspace_file', 'Rejected: unsafe path').outcome, 'rejected');
	assert.equal(workspaceToolEvidence('read_workspace_file', 'Could not read src/a.ts: unavailable').outcome, 'failed');
	assert.equal(workspaceToolEvidence('read_workspace_file', 'The agent request was paused or canceled before the file was read.').outcome, 'canceled');
	assert.equal(workspaceToolEvidence('search_workspace', 'No matching lines found for "x" in the 4 non-sensitive workspace files searched.').outcome, 'no_matches');
});

test('restored evidence is validated and capped at twenty entries', () => {
	const valid = workspaceToolEvidence('search_workspace', 'No matching lines found for "x" in the 0 files searched.');
	assert.equal(normalizeWorkspaceToolEvidence([valid, { ...valid, outputCharacters: -1 }]).length, 1);
	assert.equal(normalizeWorkspaceToolEvidence([{ ...valid, outputCharacters: 30_001 }, { ...valid, sourcePaths: ['bad\npath'] }]).length, 0);
	assert.equal(normalizeWorkspaceToolEvidence(Array.from({ length: 24 }, () => valid)).length, 20);
});

test('live progress summarizes bounded provenance without exposing queries or returned content', () => {
	const evidence = workspaceToolEvidence('search_workspace', 'Search results for "privateQuery":\nsrc/a.ts:1: confidential payload\nsrc/b.ts:2: confidential payload\nsrc/c.ts:3: confidential payload\nsrc/d.ts:4: confidential payload');
	const progress = workspaceToolProgress(evidence);
	assert.match(progress, /search_workspace: completed/);
	assert.match(progress, /src\/a\.ts, src\/b\.ts, src\/c\.ts, and 1 more/);
	assert.equal(progress.includes('privateQuery'), false);
	assert.equal(progress.includes('confidential payload'), false);
});
