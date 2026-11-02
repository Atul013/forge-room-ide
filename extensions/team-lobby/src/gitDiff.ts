import * as path from 'node:path';

const binaryMarkers = ['GIT binary patch', 'Binary files '];

export type WorkspaceGitDiff = { relativePath: string; staged?: string; unstaged?: string };
export type WorkspaceGitRepository = { rootPath: string; diffIndexWithHEAD(path: string): Promise<string>; diffWithHEAD(path: string): Promise<string> };

function isWithin(parent: string, candidate: string): boolean {
	const relative = path.relative(path.resolve(parent), path.resolve(candidate));
	return relative === '' || (!path.isAbsolute(relative) && relative !== '..' && !relative.startsWith(`..${path.sep}`));
}

/** Read one safe, non-sensitive tracked diff from the Git repository covering this workspace path. */
export async function readWorkspaceGitDiff(workspaceRoot: string, relativePath: string, repositories: WorkspaceGitRepository[], isSensitivePath: (relativePath: string) => boolean): Promise<string> {
	const normalized = relativePath.replace(/\\/g, '/');
	if (!normalized || normalized.length > 500 || normalized.startsWith('/') || /^[a-zA-Z]:/.test(normalized) || /[\u0000-\u001f\u007f]/.test(normalized) || normalized.split('/').some(part => !part || part === '.' || part === '..')) { return 'Rejected: provide a safe workspace-relative file path.'; }
	if (isSensitivePath(normalized)) { return 'Rejected: likely sensitive paths are not available through Git diff.'; }
	const absolutePath = path.resolve(workspaceRoot, normalized);
	if (!isWithin(workspaceRoot, absolutePath) || absolutePath === path.resolve(workspaceRoot)) { return 'Rejected: Git diff target resolves outside the selected workspace.'; }
	const containing = repositories.filter(repository => isWithin(repository.rootPath, absolutePath)).sort((left, right) => right.rootPath.length - left.rootPath.length)[0];
	if (!containing) { return 'No Git repository was found for the selected file.'; }
	const repositoryPath = path.relative(containing.rootPath, absolutePath).split(path.sep).join('/');
	if (!repositoryPath || repositoryPath.split('/').some(part => !part || part === '.' || part === '..')) { return 'Rejected: Git diff target is not a safe repository-relative path.'; }
	const staged = await containing.diffIndexWithHEAD(repositoryPath);
	const unstaged = await containing.diffWithHEAD(repositoryPath);
	if (/^(?:rename|copy) from /mi.test(`${staged}\n${unstaged}`)) { return `Rejected: rename or copy diff omitted for ${normalized}; it may contain content from another path.`; }
	return formatWorkspaceGitDiff({ relativePath: normalized, staged, unstaged });
}

/** Format only the requested tracked file patch and cap data before it enters a model turn. */
export function formatWorkspaceGitDiff(diff: WorkspaceGitDiff, maxCharacters = 12_000): string {
	if (!diff.relativePath || diff.relativePath.length > 500 || /[\r\n\u0000-\u001f\u007f]/.test(diff.relativePath)) { return 'Rejected: invalid workspace-relative path.'; }
	const staged = typeof diff.staged === 'string' ? diff.staged : '';
	const unstaged = typeof diff.unstaged === 'string' ? diff.unstaged : '';
	if (binaryMarkers.some(marker => staged.includes(marker) || unstaged.includes(marker))) { return `Rejected: binary diff omitted for ${diff.relativePath}.`; }
	if (!staged && !unstaged) { return `No tracked changes for ${diff.relativePath}.`; }
	const sections = [`Git diff for ${diff.relativePath}`];
	if (staged) { sections.push(`STAGED\n${staged}`); }
	if (unstaged) { sections.push(`UNSTAGED\n${unstaged}`); }
	const output = sections.join('\n\n');
	const limit = Number.isFinite(maxCharacters) ? Math.max(0, Math.floor(maxCharacters)) : 0;
	if (output.length <= limit) { return output; }
	const marker = '\n[diff truncated]';
	if (limit <= marker.length) { return output.slice(0, limit); }
	return output.slice(0, limit - marker.length) + marker;
}
