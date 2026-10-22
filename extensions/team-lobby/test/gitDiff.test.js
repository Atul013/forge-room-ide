const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const { formatWorkspaceGitDiff, readWorkspaceGitDiff } = require('../dist/gitDiff');

function repository(rootPath, staged = '', unstaged = '') {
	const calls = [];
	return {
		rootPath,
		calls,
		async diffIndexWithHEAD(file) { calls.push(['staged', file]); return staged; },
		async diffWithHEAD(file) { calls.push(['unstaged', file]); return unstaged; }
	};
}

test('reads staged and unstaged diff for exactly one workspace file', async () => {
	const workspace = path.resolve('workspace');
	const repo = repository(workspace, 'diff --cached -- src/app.ts\n+staged line', 'diff -- src/app.ts\n+working line');
	const result = await readWorkspaceGitDiff(workspace, 'src/app.ts', [repo], () => false);
	assert.match(result, /Git diff for src\/app\.ts/);
	assert.match(result, /STAGED[\s\S]*\+staged line/);
	assert.match(result, /UNSTAGED[\s\S]*\+working line/);
	assert.deepEqual(repo.calls, [['staged', 'src/app.ts'], ['unstaged', 'src/app.ts']]);
});

test('scopes nested workspaces to their containing repository and chooses the nearest nested repository', async () => {
	const workspace = path.resolve('repo', 'packages', 'app');
	const parent = repository(path.resolve('repo'));
	const nested = repository(path.resolve('repo', 'packages', 'app'));
	await readWorkspaceGitDiff(workspace, 'src/index.ts', [parent, nested], () => false);
	assert.equal(parent.calls.length, 0);
	assert.deepEqual(nested.calls, [['staged', 'src/index.ts'], ['unstaged', 'src/index.ts']]);
});

test('rejects sensitive paths and path escapes before invoking Git', async () => {
	const workspace = path.resolve('workspace');
	const repo = repository(workspace);
	assert.match(await readWorkspaceGitDiff(workspace, '.env', [repo], value => value === '.env'), /^Rejected:/);
	assert.match(await readWorkspaceGitDiff(workspace, '../outside.ts', [repo], () => false), /^Rejected:/);
	assert.match(await readWorkspaceGitDiff(workspace, 'C:\\outside.ts', [repo], () => false), /^Rejected:/);
	assert.equal(repo.calls.length, 0);
});

test('returns a no-change result and omits binary patches', async () => {
	const workspace = path.resolve('workspace');
	const empty = repository(workspace);
	assert.match(await readWorkspaceGitDiff(workspace, 'src/app.ts', [empty], () => false), /^No tracked changes/);
	const binary = formatWorkspaceGitDiff({ relativePath: 'assets/logo.png', staged: 'GIT binary patch\nliteral 10' });
	assert.match(binary, /^Rejected: binary diff omitted/);
});

test('omits rename patches so content from the previous path cannot be transferred', async () => {
	const workspace = path.resolve('workspace');
	const renamed = repository(workspace, 'diff --git a/.env b/src/settings.ts\nrename from .env\nrename to src/settings.ts\n+API_KEY=private');
	const result = await readWorkspaceGitDiff(workspace, 'src/settings.ts', [renamed], () => false);
	assert.match(result, /^Rejected: rename or copy diff omitted/);
	assert.equal(result.includes('API_KEY'), false);
});

test('caps diff output while retaining an explicit truncation marker', () => {
	const result = formatWorkspaceGitDiff({ relativePath: 'src/app.ts', unstaged: 'x'.repeat(100) }, 48);
	assert.equal(result.length, 48);
	assert.match(result, /\[diff truncated\]$/);
});
