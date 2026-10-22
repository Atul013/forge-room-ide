import * as path from 'node:path';
import { resolveWorkspaceFile } from './workspacePaths';
import { validSharedDocumentPath } from './sharedDocument';

type WorkspaceFolderPath = { name: string; uri: { fsPath: string } };

/** Creates a room path that stays unambiguous across multi-root workspaces. */
export function sharedWorkspacePath(folders: readonly WorkspaceFolderPath[], folder: WorkspaceFolderPath, filePath: string): string | undefined {
	if (!folders.includes(folder) || !path.isAbsolute(filePath)) { return undefined; }
	const relative = path.relative(folder.uri.fsPath, filePath).split(path.sep).join('/');
	if (!relative || relative === '..' || relative.startsWith('../') || path.isAbsolute(relative)) { return undefined; }
	const value = folders.length === 1 ? relative : `${folder.name}/${relative}`;
	if (!validSharedDocumentPath(value)) { return undefined; }
	if (folders.length > 1 && folders.filter(candidate => candidate.name === folder.name).length !== 1) { return undefined; }
	return value;
}

/** Resolves a room path only beneath the corresponding local workspace root. */
export async function resolveSharedWorkspacePath(folders: readonly WorkspaceFolderPath[], roomPath: string): Promise<string | undefined> {
	if (!validSharedDocumentPath(roomPath) || !folders.length) { return undefined; }
	if (folders.length === 1) { return resolveWorkspaceFile(folders[0].uri.fsPath, roomPath); }
	const [name, ...parts] = roomPath.split('/');
	const matches = folders.filter(folder => folder.name === name);
	if (matches.length !== 1 || !parts.length) { return undefined; }
	return resolveWorkspaceFile(matches[0].uri.fsPath, parts.join('/'));
}
