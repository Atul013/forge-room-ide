const test = require('node:test');
const assert = require('node:assert/strict');
const { normalizeWorkspaceToolEvidence, workspaceToolEvidence, workspaceToolProgress } = require('../dist/toolEvidence');

test('editor navigation evidence keeps only the opened workspace path', () => {
	const evidence = workspaceToolEvidence('editor_open', 'Opened workspace file: src/private-module.ts at line 17');
	assert.equal(evidence.tool, 'editor_open');
	assert.deepEqual(evidence.sourcePaths, ['src/private-module.ts']);
	assert.equal(normalizeWorkspaceToolEvidence([evidence]).length, 1);
	assert.equal(workspaceToolEvidence('editor_open', 'Rejected: file is sensitive.').outcome, 'rejected');
});

test('workspace inventory evidence records no listed paths or file contents', () => {
	const evidence = workspaceToolEvidence('workspace_list', 'Workspace files (1; paths only):\n- src/private-name.ts');
	assert.equal(evidence.tool, 'workspace_list');
	assert.deepEqual(evidence.sourcePaths, []);
	assert.equal(JSON.stringify(evidence).includes('private-name'), false);
	assert.equal(normalizeWorkspaceToolEvidence([evidence]).length, 1);
	assert.equal(workspaceToolEvidence('workspace_list', 'Workspace files (0; paths only):').outcome, 'no_matches');
});

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

test('Git status evidence records visible paths without branch names or other result text', () => {
	const evidence = workspaceToolEvidence('git_status', 'Repository . — branch private-branch: 1 staged, 1 modified, 0 untracked, 0 conflicts\n  src/a.ts (staged)\n  src/b.ts (modified)');
	assert.deepEqual(evidence.sourcePaths, ['src/a.ts', 'src/b.ts']);
	assert.equal(JSON.stringify(evidence).includes('private-branch'), false);
	assert.equal(evidence.tool, 'git_status');
	assert.equal(normalizeWorkspaceToolEvidence([evidence]).length, 1);
});

test('Git diff evidence records its selected source path but never retains the patch', () => {
	const evidence = workspaceToolEvidence('git_diff', 'Git diff for src/app.ts\nUNSTAGED\n+private source line');
	assert.equal(evidence.tool, 'git_diff');
	assert.deepEqual(evidence.sourcePaths, ['src/app.ts']);
	assert.equal(JSON.stringify(evidence).includes('private source line'), false);
	assert.equal(normalizeWorkspaceToolEvidence([evidence]).length, 1);
	assert.equal(workspaceToolEvidence('git_diff', 'No tracked changes for src/app.ts.').outcome, 'no_matches');
});

test('diagnostic evidence records file provenance without retaining diagnostic text', () => {
	const evidence = workspaceToolEvidence('diagnostics_list', 'Diagnostics for workspace (1):\nFILE src/app.ts\n- Error L1:1: private diagnostic text');
	assert.equal(evidence.tool, 'diagnostics_list');
	assert.deepEqual(evidence.sourcePaths, ['src/app.ts']);
	assert.equal(JSON.stringify(evidence).includes('private diagnostic text'), false);
	assert.equal(normalizeWorkspaceToolEvidence([evidence]).length, 1);
	assert.equal(workspaceToolEvidence('diagnostics_list', 'No diagnostics found for workspace.').outcome, 'no_matches');
});

test('symbol evidence records source paths without retaining query or symbol names', () => {
	const evidence = workspaceToolEvidence('symbols_find', 'Symbols matching "privateQuery" (1):\nFILE src/app.ts\n- Function SecretHandler:8');
	assert.equal(evidence.tool, 'symbols_find');
	assert.deepEqual(evidence.sourcePaths, ['src/app.ts']);
	assert.equal(JSON.stringify(evidence).includes('privateQuery'), false);
	assert.equal(JSON.stringify(evidence).includes('SecretHandler'), false);
	assert.equal(normalizeWorkspaceToolEvidence([evidence]).length, 1);
	assert.equal(workspaceToolEvidence('symbols_find', 'No symbols found for "absent" in the selected workspace.').outcome, 'no_matches');
});

test('test discovery evidence records only the task result size and no task labels', () => {
	const evidence = workspaceToolEvidence('tests_discover', 'Registered test tasks (1):\n- secret-task (provider: test-provider)');
	assert.equal(evidence.tool, 'tests_discover');
	assert.deepEqual(evidence.sourcePaths, []);
	assert.equal(JSON.stringify(evidence).includes('secret-task'), false);
	assert.equal(JSON.stringify(evidence).includes('test-provider'), false);
	assert.equal(normalizeWorkspaceToolEvidence([evidence]).length, 1);
	assert.equal(workspaceToolEvidence('tests_discover', 'No registered test tasks were found in the selected workspace.').outcome, 'no_matches');
});

test('test execution evidence records only the actual outcome and never the task name', () => {
	const result = 'Test task ended with exit code 1.';
	const evidence = workspaceToolEvidence('tests_run', result);
	assert.equal(evidence.tool, 'tests_run');
	assert.equal(evidence.outcome, 'failed');
	assert.deepEqual(evidence.sourcePaths, []);
	assert.equal(workspaceToolEvidence('tests_run', 'Test task ended with exit code 0.').outcome, 'success');
	assert.equal(workspaceToolEvidence('tests_run', 'Test task execution was canceled and termination was requested; its result is unverified.').outcome, 'canceled');
	assert.equal(workspaceToolEvidence('tests_run', 'The approved test task exceeded the 10-minute limit and was terminated; its result is unverified.').outcome, 'failed');
	assert.equal(normalizeWorkspaceToolEvidence([evidence]).length, 1);
});

test('debugger frame evidence keeps source paths and output size but omits frame names', () => {
	const evidence = workspaceToolEvidence('debugger_frames', 'Active debug stack (1 frames; variables are not included):\nFRAME secretFunction — src/app.ts:12');
	assert.deepEqual(evidence.sourcePaths, ['src/app.ts']);
	assert.equal(evidence.outcome, 'success');
	assert.equal(JSON.stringify(evidence).includes('secretFunction'), false);
	assert.equal(workspaceToolEvidence('debugger_frames', 'No workspace stack frames are available for the active paused thread.').outcome, 'no_matches');
});

test('terminal output evidence stores only capability, size and outcome', () => {
	const evidence = workspaceToolEvidence('terminal_last_output', 'COMMAND: npm test\naccess_token=secret-value\nTests failed');
	assert.equal(evidence.tool, 'terminal_last_output');
	assert.deepEqual(evidence.sourcePaths, []);
	assert.equal(evidence.outcome, 'success');
	assert.equal(JSON.stringify(evidence).includes('secret-value'), false);
	assert.equal(normalizeWorkspaceToolEvidence([evidence]).length, 1);
	assert.equal(workspaceToolEvidence('terminal_last_output', 'No captured terminal output is available.').outcome, 'no_matches');
});

test('workspace edit evidence stores only the edited path and result size', () => {
	const evidence = workspaceToolEvidence('apply_workspace_edit', 'Applied workspace edit to src/main.ts\nThe result is unsaved.', '2026-01-01T00:00:00.000Z');
	assert.equal(evidence.tool, 'apply_workspace_edit');
	assert.deepEqual(evidence.sourcePaths, ['src/main.ts']);
	assert.equal(JSON.stringify(evidence).includes('result is unsaved'), false);
	assert.equal(normalizeWorkspaceToolEvidence([evidence]).length, 1);
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
