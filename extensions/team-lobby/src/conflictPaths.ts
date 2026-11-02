export function updateReportedConflictPaths(reported: Set<string>, paths: string[], enabled: boolean, connected: boolean) {
	if (!enabled || !connected) { reported.clear(); return []; }
	const current = new Set(paths.filter(Boolean).slice(0, 100));
	const added = [...current].filter(file => !reported.has(file));
	for (const file of current) { reported.add(file); }
	for (const file of reported) { if (!current.has(file)) { reported.delete(file); } }
	return added;
}
