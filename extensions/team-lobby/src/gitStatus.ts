import path from 'path';

export type GitStatusChange = { absolutePath: string; status: 'staged' | 'modified' | 'untracked' | 'conflict' };
export type GitStatusRepository = { rootPath: string; branch?: string; changes: GitStatusChange[] };

function isWithin(parent: string, candidate: string): boolean {
	const relative = path.relative(path.resolve(parent), path.resolve(candidate));
	return relative === '' || (relative !== '..' && !relative.startsWith(`..${path.sep}`) && !path.isAbsolute(relative));
}

/** Format a bounded Git summary scoped to the selected workspace, with no diff or file content. */
export function formatWorkspaceGitStatus(workspaceRoot: string, repositories: GitStatusRepository[], isSensitivePath: (relativePath: string) => boolean): string {
	const relevant = repositories.filter(repository => isWithin(workspaceRoot, repository.rootPath) || isWithin(repository.rootPath, workspaceRoot));
	if (!relevant.length) { return 'No Git repository was found inside the selected workspace.'; }
	const lines: string[] = [];
	let listedPaths = 0;
	for (const repository of relevant) {
		const relativeRoot = path.relative(workspaceRoot, repository.rootPath).replace(/\\/g, '/');
		const repoLabel = relativeRoot && relativeRoot !== '..' && !relativeRoot.startsWith('../') && !path.isAbsolute(relativeRoot) ? relativeRoot : '.';
		const visible = repository.changes.map(change => {
			const relative = path.relative(workspaceRoot, change.absolutePath).replace(/\\/g, '/');
			if (!relative || relative === '..' || relative.startsWith('../') || path.isAbsolute(relative) || isSensitivePath(relative)) { return undefined; }
			return { path: relative, status: change.status };
		}).filter((change): change is { path: string; status: GitStatusChange['status'] } => !!change);
		const count = (status: GitStatusChange['status']) => visible.filter(change => change.status === status).length;
		lines.push(`Repository ${repoLabel} - branch ${repository.branch || '(detached or unborn HEAD)'}: ${count('staged')} staged, ${count('modified')} modified, ${count('untracked')} untracked, ${count('conflict')} conflicts`);
		const remaining = Math.max(0, 40 - listedPaths);
		for (const change of visible.slice(0, remaining)) { lines.push(`  ${change.path} (${change.status})`); listedPaths++; }
		if (visible.length > remaining) { lines.push(`  ${visible.length - remaining} additional changes omitted`); }
		if (listedPaths >= 40 || lines.length >= 48) { if (relevant.indexOf(repository) < relevant.length - 1) { lines.push('Additional repositories omitted.'); } break; }
	}
	return lines.join('\n').slice(0, 6000);
}
