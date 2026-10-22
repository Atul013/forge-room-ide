export type WorkspaceSymbolResult = { path: string; name: string; kind: string; containerName?: string; line?: number };

function clean(value: string, maxLength: number): string {
	return value.replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/g, ' ').replace(/[\r\n\t ]+/g, ' ').trim().slice(0, maxLength);
}

function safeRelativePath(value: string): boolean {
	const normalized = value.replace(/\\/g, '/');
	return !!normalized && normalized.length <= 500 && !normalized.startsWith('/') && !/^[a-zA-Z]:/.test(normalized)
		&& !/[\r\n\u0000-\u001f\u007f]/.test(normalized) && !normalized.split('/').some(part => !part || part === '.' || part === '..');
}

/** Format bounded workspace symbol names and source locations without reading source text. */
export function formatWorkspaceSymbols(query: string, symbols: WorkspaceSymbolResult[], maxItems = 100, maxCharacters = 12_000): string {
	const label = clean(query, 160) || '(empty query)';
	const valid = symbols.filter(symbol => safeRelativePath(symbol.path) && !!clean(symbol.name, 200) && !!clean(symbol.kind, 40)
		&& (symbol.line === undefined || (Number.isSafeInteger(symbol.line) && symbol.line >= 1)))
		.slice(0, Number.isFinite(maxItems) ? Math.max(0, Math.floor(maxItems)) : 0);
	if (!valid.length) { return `No symbols found for ${JSON.stringify(label)} in the selected workspace.`; }
	const lines = [`Symbols matching ${JSON.stringify(label)} (${valid.length}${symbols.length > valid.length ? ', more omitted' : ''}):`];
	for (const symbol of valid) {
		const container = symbol.containerName ? ` in ${clean(symbol.containerName, 200)}` : '';
		const location = symbol.line === undefined ? '' : `:${symbol.line}`;
		lines.push(`FILE ${symbol.path}\n- ${clean(symbol.kind, 40)} ${clean(symbol.name, 200)}${container}${location}`);
	}
	const output = lines.join('\n');
	const limit = Number.isFinite(maxCharacters) ? Math.max(0, Math.floor(maxCharacters)) : 0;
	if (output.length <= limit) { return output; }
	const marker = '\n[symbols truncated]';
	if (limit <= marker.length) { return output.slice(0, limit); }
	return output.slice(0, limit - marker.length) + marker;
}
