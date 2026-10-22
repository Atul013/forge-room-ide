import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { resolveTaskWorktree, sameFileSystemPath, type TaskWorktreeRecord } from './taskWorktreeIdentity';

const execFileAsync = promisify(execFile);

export type TaskWorktreeStatus = { worktree: TaskWorktreeRecord; changedPaths: string[]; omittedPathCount: number };

async function git(cwd: string, ...args: string[]) {
	return execFileAsync('git', ['-C', cwd, ...args], { windowsHide: true, timeout: 15000, maxBuffer: 1024 * 1024 });
}

export async function inspectTaskWorktree(workspacePath: string, taskBranch: string, cached?: TaskWorktreeRecord): Promise<TaskWorktreeStatus> {
	const resolved = await resolveTaskWorktree(workspacePath, taskBranch, cached);
	if (!resolved) { throw new Error('The assigned task worktree could not be verified in this repository.'); }
	const result = await git(resolved.worktree.path, 'status', '--porcelain=v1', '--untracked-files=all', '-z');
	const entries = result.stdout.split('\0');
	const paths: string[] = [];
	for (let index = 0; index < entries.length; index++) {
		const entry = entries[index];
		if (!entry) { continue; }
		const status = entry.slice(0, 2);
		paths.push(entry.length > 3 ? entry.slice(3) : entry);
		if (status.includes('R') || status.includes('C')) { index++; }
	}
	return { worktree: resolved.worktree, changedPaths: paths.slice(0, 20), omittedPathCount: Math.max(0, paths.length - 20) };
}

/** Remove only the reviewed task worktree. Git refuses removal if it became dirty after inspection. */
export async function removeTaskWorktree(workspacePath: string, taskBranch: string, cached: TaskWorktreeRecord, expectedPath: string): Promise<{ removed: true; worktree: TaskWorktreeRecord } | { removed: false; worktree: TaskWorktreeRecord; changedPaths: string[]; omittedPathCount: number }> {
	const resolved = await resolveTaskWorktree(workspacePath, taskBranch, cached);
	if (!resolved || !sameFileSystemPath(resolved.worktree.path, expectedPath)) { throw new Error('The task worktree changed after it was reviewed. Inspect it again before removing.'); }
	const inspected = await inspectTaskWorktree(workspacePath, taskBranch, resolved.worktree);
	if (!sameFileSystemPath(inspected.worktree.path, expectedPath)) { throw new Error('The task worktree changed after it was reviewed. Inspect it again before removing.'); }
	if (inspected.changedPaths.length || inspected.omittedPathCount) { return { removed: false, ...inspected }; }
	await git(workspacePath, 'worktree', 'remove', expectedPath);
	return { removed: true, worktree: resolved.worktree };
}
