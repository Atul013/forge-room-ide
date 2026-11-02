export type ExactTextEdit = { text: string; start: number; end: number };
export type WorkspaceEditCheckpointInput = { id: string; root: string; createdAt: string; path: string; before: string; after: string; task?: { id: string; title: string } };

/** Prepare one exact, unique text replacement without mutating the source. */
export function prepareExactTextEdit(current: string, oldText: string, newText: string, maxFileCharacters = 256 * 1024, maxEditCharacters = 12_000): ExactTextEdit | undefined {
	if (current.length > maxFileCharacters || !oldText.length || oldText.length > maxEditCharacters || newText.length > maxEditCharacters || oldText === newText) { return undefined; }
	const start = current.indexOf(oldText);
	if (start < 0 || current.indexOf(oldText, start + 1) >= 0) { return undefined; }
	return { text: current.slice(0, start) + newText + current.slice(start + oldText.length), start, end: start + oldText.length };
}

/** Store only the touched file's before/after text so the standard rollback guard can protect later edits. */
export function workspaceEditCheckpoint(input: WorkspaceEditCheckpointInput) {
	return {
		id: input.id,
		root: input.root,
		createdAt: input.createdAt,
		summary: `Approved workspace edit: ${input.path}`.slice(0, 240),
		...(input.task ? { task: input.task } : {}),
		evidence: [] as Array<{ name: string; completedAt: string }>,
		files: [{ path: input.path, before: input.before, after: input.after }]
	};
}
