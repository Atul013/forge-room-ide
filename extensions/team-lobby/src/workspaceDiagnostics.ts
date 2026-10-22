export type WorkspaceDiagnostic = {
	path: string;
	severity: 'Error' | 'Warning' | 'Information' | 'Hint';
	line: number;
	column: number;
	code?: string;
	source?: string;
	message: string;
};

function clean(value: string, maxLength: number): string {
	return value.replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/g, ' ').replace(/[\r\n\t ]+/g, ' ').trim().slice(0, maxLength);
}

function safeRelativePath(value: string): boolean {
	const normalized = value.replace(/\\/g, '/');
	return !!normalized && normalized.length <= 500 && !normalized.startsWith('/') && !/^[a-zA-Z]:/.test(normalized)
		&& !/[\r\n\u0000-\u001f\u007f]/.test(normalized) && !normalized.split('/').some(part => !part || part === '.' || part === '..');
}

/** Format content-free source coordinates and bounded diagnostics for an approved model tool call. */
export function formatWorkspaceDiagnostics(scope: string, diagnostics: WorkspaceDiagnostic[], maxItems = 100, maxCharacters = 12_000): string {
	const label = clean(scope, 500) || 'selected workspace';
	const valid = diagnostics.filter(item => safeRelativePath(item.path)
		&& ['Error', 'Warning', 'Information', 'Hint'].includes(item.severity)
		&& Number.isSafeInteger(item.line) && item.line >= 1
		&& Number.isSafeInteger(item.column) && item.column >= 1)
		.slice(0, Number.isFinite(maxItems) ? Math.max(0, Math.floor(maxItems)) : 0);
	if (!valid.length) { return `No diagnostics found for ${label}.`; }
	const lines = [`Diagnostics for ${label} (${valid.length}${diagnostics.length > valid.length ? ', more omitted' : ''}):`];
	for (const item of valid) {
		const code = item.code ? ` [${clean(item.code, 80)}]` : '';
		const source = item.source ? ` (${clean(item.source, 80)})` : '';
		lines.push(`FILE ${item.path}\n- ${item.severity} L${item.line}:${item.column}${code}${source}: ${clean(item.message, 500) || '(no message)'}`);
	}
	const text = lines.join('\n');
	const limit = Number.isFinite(maxCharacters) ? Math.max(0, Math.floor(maxCharacters)) : 0;
	if (text.length <= limit) { return text; }
	const marker = '\n[diagnostics truncated]';
	if (limit <= marker.length) { return text.slice(0, limit); }
	return text.slice(0, limit - marker.length) + marker;
}
