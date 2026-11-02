import * as fs from 'node:fs';
import * as path from 'node:path';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

export type TaskWorktreeRecord = { path: string; branch: string; createdAt: string };

async function git(cwd: string, ...args: string[]) {
	return (await execFileAsync('git', ['-C', cwd, ...args], { windowsHide: true, timeout: 10000 })).stdout.trim();
}

async function gitCommonDir(root: string) {
	return fs.realpathSync(path.resolve(root, await git(root, 'rev-parse', '--git-common-dir')));
}

export function sameFileSystemPath(left: string, right: string, platform = process.platform) {
	const a = path.resolve(left);
	const b = path.resolve(right);
	return platform === 'win32' ? a.toLowerCase() === b.toLowerCase() : a === b;
}

/** A task worktree must be a linked worktree on the exact branch assigned to that task. */
export function isTaskWorktreeIdentity(taskBranch: string | undefined, recordedBranch: string, actualBranch: string, gitDir: string, commonDir: string, platform = process.platform) {
	return !!taskBranch && recordedBranch === taskBranch && actualBranch === taskBranch && !sameFileSystemPath(gitDir, commonDir, platform);
}

export function findTaskWorktree(porcelain: string, taskBranch: string): { path: string; branch: string } | undefined {
	for (const entry of porcelain.split(/\r?\n\r?\n/)) {
		const worktreePath = entry.match(/^worktree (.+)$/m)?.[1];
		const branch = entry.match(/^branch refs\/heads\/(.+)$/m)?.[1];
		if (worktreePath && path.isAbsolute(worktreePath) && branch === taskBranch) { return { path: worktreePath, branch }; }
	}
	return undefined;
}

export async function resolveTaskWorktree(workspacePath: string, taskBranch: string | undefined, cached?: TaskWorktreeRecord): Promise<{ worktree: TaskWorktreeRecord; recovered: boolean } | undefined> {
	try {
		const repositoryRoot = await git(workspacePath, 'rev-parse', '--show-toplevel');
		const commonDir = await gitCommonDir(repositoryRoot);
		const validate = async (candidate: TaskWorktreeRecord | undefined): Promise<TaskWorktreeRecord | undefined> => {
			if (!candidate || typeof candidate.path !== 'string' || !path.isAbsolute(candidate.path) || typeof candidate.branch !== 'string' || !fs.existsSync(candidate.path)) { return undefined; }
			try {
				const canonicalPath = fs.realpathSync(candidate.path);
				const root = await git(canonicalPath, 'rev-parse', '--show-toplevel');
				const [actualBranch, gitDirText, actualCommonDir] = await Promise.all([
					git(root, 'rev-parse', '--abbrev-ref', 'HEAD'),
					git(root, 'rev-parse', '--git-dir'),
					gitCommonDir(root)
				]);
				const gitDir = fs.realpathSync(path.resolve(root, gitDirText));
				if (!sameFileSystemPath(root, canonicalPath) || !sameFileSystemPath(actualCommonDir, commonDir) || !isTaskWorktreeIdentity(taskBranch, candidate.branch, actualBranch, gitDir, actualCommonDir)) { return undefined; }
				return { path: canonicalPath, branch: candidate.branch, createdAt: typeof candidate.createdAt === 'string' ? candidate.createdAt : new Date().toISOString() };
			} catch { return undefined; }
		};
		const existing = await validate(cached);
		if (existing) { return { worktree: existing, recovered: false }; }
		if (!taskBranch) { return undefined; }
		const registered = await git(repositoryRoot, 'worktree', 'list', '--porcelain');
		const discovered = findTaskWorktree(registered, taskBranch);
		if (!discovered) { return undefined; }
		const recovered = await validate({ ...discovered, createdAt: new Date().toISOString() });
		return recovered ? { worktree: recovered, recovered: true } : undefined;
	} catch { return undefined; }
}
