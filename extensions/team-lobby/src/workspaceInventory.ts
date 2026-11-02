import { isSensitiveContextTarget } from './contextFiles';

const controlCharacters = /[\u0000-\u001f\u007f]/;

function safeWorkspacePath(value: unknown): value is string {
	if (typeof value !== 'string' || !value.trim() || value.length > 500 || controlCharacters.test(value)) { return false; }
	const normalized = value.replace(/\\/g, '/');
	return !normalized.startsWith('/') && !/^[a-zA-Z]:/.test(normalized) && normalized.split('/').every(part => part && part !== '.' && part !== '..');
}

/** Format only bounded relative file names; no file metadata or contents are included. */
export function formatWorkspaceInventory(paths: readonly string[], maxFiles = 250, maxCharacters = 12_000, hasMore = false): string {
	const unique: string[] = [];
	const seen = new Set<string>();
	const limit = Math.max(0, Math.floor(maxFiles));
	for (const value of paths) {
		if (unique.length >= limit) { break; }
		if (!safeWorkspacePath(value) || isSensitiveContextTarget(value, value)) { continue; }
		const filePath = value.replace(/\\/g, '/');
		if (!seen.has(filePath)) { seen.add(filePath); unique.push(filePath); }
	}
	const header = `Workspace files (${unique.length}${hasMore || paths.length > unique.length ? ', additional files omitted' : ''}; paths only):`;
	const lines = [header, ...unique.map(filePath => `- ${filePath}`)];
	const output = lines.join('\n');
	if (output.length <= maxCharacters) { return output; }
	const suffix = '\n[truncated]';
	return `${output.slice(0, Math.max(0, maxCharacters - suffix.length))}${suffix}`.slice(0, maxCharacters);
}
