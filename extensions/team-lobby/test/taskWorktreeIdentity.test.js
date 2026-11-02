const test = require('node:test');
const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { findTaskWorktree, isTaskWorktreeIdentity, resolveTaskWorktree, sameFileSystemPath } = require('../dist/taskWorktreeIdentity');

function git(cwd, ...args) {
	return execFileSync('git', ['-C', cwd, ...args], { encoding: 'utf8', windowsHide: true }).trim();
}

test('task worktree identity matches the actual linked worktree and assigned branch', async () => {
	const temporaryRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'forge room git worktree-'));
	const repository = path.join(temporaryRoot, 'repository');
	const worktree = path.join(temporaryRoot, 'task worktree');
	try {
		fs.mkdirSync(repository);
		execFileSync('git', ['init', '-b', 'main', repository], { stdio: 'ignore', windowsHide: true });
		git(repository, 'config', 'user.name', 'Forge Test');
		git(repository, 'config', 'user.email', 'forge-test@example.invalid');
		fs.writeFileSync(path.join(repository, 'README.md'), 'worktree fixture\n');
		git(repository, 'add', 'README.md');
		git(repository, 'commit', '-m', 'fixture');
		git(repository, 'worktree', 'add', '-b', 'forge-room/task-check', worktree, 'HEAD');

		const top = git(worktree, 'rev-parse', '--show-toplevel');
		const branch = git(worktree, 'rev-parse', '--abbrev-ref', 'HEAD');
		const gitDir = fs.realpathSync(path.resolve(worktree, git(worktree, 'rev-parse', '--git-dir')));
		const commonDir = fs.realpathSync(path.resolve(worktree, git(worktree, 'rev-parse', '--git-common-dir')));
		const worktreeList = git(repository, 'worktree', 'list', '--porcelain');
		assert.equal(sameFileSystemPath(top, worktree), true);
		const discovered = findTaskWorktree(worktreeList, 'forge-room/task-check');
		assert.ok(discovered);
		assert.equal(discovered.branch, 'forge-room/task-check');
		assert.equal(sameFileSystemPath(discovered.path, worktree), true);
		assert.equal(findTaskWorktree(worktreeList, 'forge-room/missing'), undefined);
		assert.equal(isTaskWorktreeIdentity('forge-room/task-check', 'forge-room/task-check', branch, gitDir, commonDir), true);
		assert.equal(isTaskWorktreeIdentity('forge-room/another-task', 'forge-room/task-check', branch, gitDir, commonDir), false);
		assert.equal(isTaskWorktreeIdentity('forge-room/task-check', 'forge-room/task-check', 'main', gitDir, commonDir), false);
		assert.equal(isTaskWorktreeIdentity('forge-room/task-check', 'forge-room/task-check', branch, commonDir, commonDir), false);

		const cachedWrongPath = { path: repository, branch: 'main', createdAt: 'old-record' };
		const recovery = await resolveTaskWorktree(repository, 'forge-room/task-check', cachedWrongPath);
		assert.ok(recovery?.recovered);
		assert.equal(sameFileSystemPath(recovery.worktree.path, worktree), true);
		assert.equal(recovery.worktree.branch, 'forge-room/task-check');
		const cachedValidPath = await resolveTaskWorktree(repository, 'forge-room/task-check', { path: worktree, branch, createdAt: 'valid-record' });
		assert.equal(cachedValidPath?.recovered, false);
		assert.equal(sameFileSystemPath(cachedValidPath.worktree.path, worktree), true);
		assert.equal(await resolveTaskWorktree(repository, 'forge-room/not-present', cachedWrongPath), undefined);
	} finally {
		try { if (fs.existsSync(repository)) { execFileSync('git', ['-C', repository, 'worktree', 'remove', '--force', worktree], { stdio: 'ignore', windowsHide: true }); } } catch { /* The fixture may fail before registering the worktree. */ }
		fs.rmSync(temporaryRoot, { recursive: true, force: true });
	}
});

test('path comparisons follow the host operating system rules', () => {
	assert.equal(sameFileSystemPath('C:\\Forge\\Task', 'c:\\forge\\task', 'win32'), true);
	assert.equal(sameFileSystemPath('/repo/Task', '/repo/task', 'linux'), false);
});
