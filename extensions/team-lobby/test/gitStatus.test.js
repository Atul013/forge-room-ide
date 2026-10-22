const test = require('node:test');
const assert = require('node:assert/strict');
const path = require('node:path');
const { formatWorkspaceGitStatus } = require('../dist/gitStatus');

test('Git status clips parent repository changes to a nested selected workspace', () => {
	const root = path.resolve('workspace');
	const repository = path.resolve('..');
	const result = formatWorkspaceGitStatus(root, [{
		rootPath: repository,
		branch: 'feature/demo',
		changes: [
			{ absolutePath: path.join(root, 'src', 'main.ts'), status: 'modified' },
			{ absolutePath: path.join(repository, 'sibling', 'secret.txt'), status: 'modified' }
		]
	}], () => false);
	assert.match(result, /src\/main\.ts \(modified\)/);
	assert.equal(result.includes('sibling'), false);
	assert.match(result, /1 modified/);
});

test('Git status omits sensitive paths, excludes unrelated repositories, and caps listed changes', () => {
	const root = path.resolve('workspace');
	const changes = Array.from({ length: 45 }, (_, index) => ({ absolutePath: path.join(root, `src/file-${index}.ts`), status: 'untracked' }));
	changes.push({ absolutePath: path.join(root, '.env'), status: 'untracked' });
	const result = formatWorkspaceGitStatus(root, [
		{ rootPath: root, branch: 'main', changes },
		{ rootPath: path.resolve('unrelated'), branch: 'outside', changes: [] }
	], relative => relative.split('/').includes('.env'));
	assert.equal((result.match(/\(untracked\)/g) || []).length, 40);
	assert.equal(result.includes('.env'), false);
	assert.equal(result.includes('outside'), false);
	assert.match(result, /additional changes omitted/);
});
