export type AgentSkillMetadata = { name: string; description: string };

/** Parse the required Agent Skills fields without evaluating YAML or scripts. */
export function parseAgentSkill(markdown: string, directoryName: string): { metadata?: AgentSkillMetadata; body?: string; error?: string } {
	if (markdown.length > 64_000) { return { error: 'SKILL.md exceeds 64 KB.' }; }
	const normalized = markdown.replace(/^\uFEFF/, '');
	const match = normalized.match(/^---\s*\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)([\s\S]*)$/);
	if (!match) { return { error: 'Missing YAML frontmatter.' }; }
	const fields = new Map<string, string>();
	const lines = match[1].split(/\r?\n/);
	for (let index = 0; index < lines.length; index++) {
		const line = lines[index];
		if (!line.trim() || /^\s*#/.test(line)) { continue; }
		const field = line.match(/^([a-z][a-z0-9_-]*):\s*(.*?)\s*$/i);
		if (!field) { continue; }
		const [, key, raw] = field;
		if (fields.has(key)) { return { error: `Duplicate frontmatter field: ${key}.` }; }
		let value = raw;
		if (key === 'description' && /^(?:\||>)[+-]?$/.test(value)) {
			const content: string[] = [];
			while (index + 1 < lines.length && /^\s+\S/.test(lines[index + 1])) { content.push(lines[++index].trim()); }
			value = value.startsWith('|') ? content.join('\n') : content.join(' ');
		}
		if ((value.startsWith('"') && value.endsWith('"')) || (value.startsWith("'") && value.endsWith("'"))) { value = value.slice(1, -1); }
		fields.set(key, value);
	}
	const name = fields.get('name');
	const description = fields.get('description');
	if (!name || !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(name) || name.length > 64) { return { error: 'Skill name must use lowercase letters, numbers, and single hyphens (up to 64 characters).' }; }
	if (name !== directoryName) { return { error: 'Skill name must match its parent directory.' }; }
	if (!description || description.length > 1024) { return { error: 'Skill description is required and limited to 1,024 characters.' }; }
	const body = match[2].trim();
	if (!body) { return { error: 'Skill instructions are empty.' }; }
	return { metadata: { name, description }, body: body.slice(0, 20_000) };
}
