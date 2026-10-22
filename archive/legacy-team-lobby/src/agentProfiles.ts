export type AgentProfile = { name: string; description: string; instructions: string };

export const agentProfilesPath = '.forge-room/agent-profiles.json';
export const maxAgentProfiles = 12;
export const maxAgentProfileInstructions = 8_000;

export function parseAgentProfiles(text: string): { profiles?: AgentProfile[]; error?: string } {
	if (text.length > 64_000) { return { error: 'Profile file exceeds 64 KB.' }; }
	let document: unknown;
	try { document = JSON.parse(text); }
	catch { return { error: 'Profile file must contain valid JSON.' }; }
	if (!document || typeof document !== 'object' || Array.isArray(document)) { return { error: 'Profile file must be a JSON object.' }; }
	const root = document as Record<string, unknown>;
	if (Object.keys(root).some(key => !['format', 'version', 'profiles'].includes(key))
		|| root.format !== 'forge-room-agent-profiles' || root.version !== 1
		|| !Array.isArray(root.profiles) || root.profiles.length > maxAgentProfiles) {
		return { error: 'Unsupported profile format or too many profiles (maximum 12).' };
	}
	const profiles: AgentProfile[] = [];
	const names = new Set<string>();
	for (const item of root.profiles) {
		if (!item || typeof item !== 'object' || Array.isArray(item)) { return { error: 'Each profile must be an object.' }; }
		const profile = item as Record<string, unknown>;
		if (Object.keys(profile).some(key => !['name', 'description', 'instructions'].includes(key))
			|| typeof profile.name !== 'string' || profile.name.trim().length === 0 || profile.name.length > 60
			|| typeof profile.description !== 'string' || profile.description.length > 240
			|| typeof profile.instructions !== 'string' || profile.instructions.trim().length === 0 || profile.instructions.length > maxAgentProfileInstructions) {
			return { error: 'Profiles require a name, description, and 1–8,000 characters of instructions.' };
		}
		const name = profile.name.trim();
		const key = name.toLocaleLowerCase();
		if (names.has(key)) { return { error: 'Profile names must be unique, ignoring case.' }; }
		names.add(key);
		profiles.push({ name, description: profile.description.trim(), instructions: profile.instructions.trim() });
	}
	return { profiles };
}
