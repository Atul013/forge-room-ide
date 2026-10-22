import { realpath } from 'node:fs/promises';
import * as fs from 'node:fs';
import * as path from 'node:path';

function containsPath(root: string, candidate: string) {
	const relative = path.relative(root, candidate);
	return relative === '' || (!path.isAbsolute(relative) && relative !== '..' && !relative.startsWith(`..${path.sep}`));
}

export type ResolvedWorkspaceFile = { absolutePath: string; workspaceRelativePath: string };

export async function resolveWorkspaceFileDetails(workspaceRoot: string, relativePath: string): Promise<ResolvedWorkspaceFile | undefined> {
	if (!relativePath || relativePath.includes('\0') || path.isAbsolute(relativePath) || relativePath.split(/[\\/]/).includes('..')) { return undefined; }
	try {
		const root = await realpath(workspaceRoot);
		const candidate = await realpath(path.resolve(root, relativePath));
		if (!containsPath(root, candidate)) { return undefined; }
		return { absolutePath: candidate, workspaceRelativePath: path.relative(root, candidate).split(path.sep).join('/') };
	} catch {
		return undefined;
	}
}

export async function resolveWorkspaceFile(workspaceRoot: string, relativePath: string): Promise<string | undefined> {
	return (await resolveWorkspaceFileDetails(workspaceRoot, relativePath))?.absolutePath;
}

/** Resolve a file destination only when its parent is inside the workspace and the target is not a symlink. */
export async function resolveWorkspaceFileDestination(workspaceRoot: string, relativePath: string): Promise<string | undefined> {
	if (!relativePath || relativePath.includes('\0') || path.isAbsolute(relativePath) || relativePath.split(/[\\/]/).some(part => !part || part === '.' || part === '..')) { return undefined; }
	try {
		const root = await realpath(workspaceRoot);
		const target = path.resolve(root, relativePath);
		const parent = await realpath(path.dirname(target));
		if (!containsPath(root, parent)) { return undefined; }
		const destination = path.join(parent, path.basename(target));
		try {
			const stat = await fs.promises.lstat(destination);
			if (stat.isSymbolicLink() || !stat.isFile()) { return undefined; }
		} catch (error) {
			if ((error as NodeJS.ErrnoException).code !== 'ENOENT') { return undefined; }
		}
		return destination;
	} catch {
		return undefined;
	}
}
