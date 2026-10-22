const test = require('node:test');
const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { inspectTaskWorktree, removeTaskWorktree } = require('../dist/taskWorktreeManagement');

function git(cwd, ...args) {
	return execFileSync('git', ['-C', cwd, ...args], { encoding: 'utf8', windowsHide: true }).trim();
}

test('worktree cleanup refuses dirty files, removes the reviewed clean worktree, and keeps its branch', async () => {
	const temporaryRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'forge room worktree cleanup-'));
	const repository = path.join(temporaryRoot, 'repository');
	const worktree = path.join(temporaryRoot, 'task worktree');
	const branch = 'forge-room/cleanup-check';
	try {
		fs.mkdirSync(repository);
		execFileSync('git', ['init', '-b', 'main', repository], { stdio: 'ignore', windowsHide: true });
		git(repository, 'config', 'user.name', 'Forge Test');
		git(repository, 'config', 'user.email', 'forge-test@example.invalid');
		fs.writeFileSync(path.join(repository, 'README.md'), 'worktree fixture\n');
		git(repository, 'add', 'README.md');
		git(repository, 'commit', '-m', 'fixture');
		git(repository, 'worktree', 'add', '-b', branch, worktree, 'HEAD');
		const cached = { path: worktree, branch, createdAt: new Date().toISOString() };

		const clean = await inspectTaskWorktree(repository, branch, cached);
		assert.deepEqual(clean.changedPaths, []);
		fs.writeFileSync(path.join(worktree, 'unsaved.txt'), 'keep me\n');
		const dirty = await inspectTaskWorktree(repository, branch, cached);
		assert.deepEqual(dirty.changedPaths, ['unsaved.txt']);
		const refused = await removeTaskWorktree(repository, branch, cached, worktree);
		assert.equal(refused.removed, false);
		assert.deepEqual(refused.changedPaths, ['unsaved.txt']);
		assert.equal(fs.existsSync(worktree), true);

		fs.unlinkSync(path.join(worktree, 'unsaved.txt'));
		const removed = await removeTaskWorktree(repository, branch, cached, worktree);
		assert.equal(removed.removed, true);
		assert.equal(fs.existsSync(worktree), false);
		execFileSync('git', ['-C', repository, 'show-ref', '--verify', '--quiet', `refs/heads/${branch}`], { stdio: 'ignore', windowsHide: true });
	} finally {
		try { if (fs.existsSync(repository)) { execFileSync('git', ['-C', repository, 'worktree', 'remove', '--force', worktree], { stdio: 'ignore', windowsHide: true }); } } catch { /* The fixture may fail before registering the worktree. */ }
		fs.rmSync(temporaryRoot, { recursive: true, force: true });
	}
});
