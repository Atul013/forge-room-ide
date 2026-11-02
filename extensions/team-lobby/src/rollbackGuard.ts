export type RollbackGuardFile = { path: string; after: string; created?: boolean };
export type RollbackGuardObservation = { exists: boolean; insideWorkspace: boolean; createdFileIsSymlink: boolean; content?: string };
export type RollbackConflict = { path: string; reason: 'missing' | 'outside-workspace' | 'unsafe-created-file' | 'changed' };

/** Fail the whole restore if any target is missing, escaped, or no longer matches the applied Build state. */
export function findRollbackConflict(
	files: readonly RollbackGuardFile[],
	observe: (file: RollbackGuardFile) => RollbackGuardObservation
): RollbackConflict | undefined {
	for (const file of files) {
		const state = observe(file);
		if (!state.exists) { return { path: file.path, reason: 'missing' }; }
		if (!state.insideWorkspace) { return { path: file.path, reason: 'outside-workspace' }; }
		if (file.created && state.createdFileIsSymlink) { return { path: file.path, reason: 'unsafe-created-file' }; }
		if (state.content !== file.after) { return { path: file.path, reason: 'changed' }; }
	}
	return undefined;
}
