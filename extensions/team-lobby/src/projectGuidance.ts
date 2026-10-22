export type ProjectGuidance = { path: string; content: string };

export const maxProjectGuidanceFiles = 10;
export const maxProjectGuidanceCandidates = 100;
export const maxProjectGuidanceFileBytes = 256 * 1024;
export const maxProjectGuidanceFileCharacters = 12_000;
export const maxProjectGuidanceCharacters = 48_000;

export function isProjectGuidancePath(value: string): boolean {
	if (!value || value.length > 500 || value.includes('\0') || value.startsWith('/') || /^[a-zA-Z]:/.test(value)) { return false; }
	const normalized = value.replace(/\\/g, '/');
	const segments = normalized.split('/');
	if (segments.some(segment => !segment || segment === '.' || segment === '..')) { return false; }
	if (normalized === '.forge-room/instructions.md' || segments.at(-1) === 'AGENTS.md') {
		return !segments.some(segment => ['.git', 'node_modules', 'out', 'dist', '.build'].includes(segment.toLowerCase()));
	}
	return false;
}

export function boundProjectGuidance(sources: ProjectGuidance[]): { sources: ProjectGuidance[]; omittedCount: number } {
	const bounded: ProjectGuidance[] = [];
	const seen = new Set<string>();
	let characters = 0;
	let omittedCount = 0;
	for (const source of sources) {
		const key = source.path.toLowerCase();
		if (!isProjectGuidancePath(source.path) || seen.has(key) || bounded.length >= maxProjectGuidanceFiles || characters >= maxProjectGuidanceCharacters) {
			omittedCount++;
			continue;
		}
		seen.add(key);
		const remaining = maxProjectGuidanceCharacters - characters;
		const content = source.content.slice(0, Math.min(maxProjectGuidanceFileCharacters, remaining));
		if (content.length < source.content.length) { omittedCount++; }
		if (!content) { omittedCount++; continue; }
		bounded.push({ path: source.path, content });
		characters += content.length;
	}
	return { sources: bounded, omittedCount };
}
