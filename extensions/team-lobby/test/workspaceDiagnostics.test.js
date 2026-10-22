const test = require('node:test');
const assert = require('node:assert/strict');
const { formatWorkspaceDiagnostics } = require('../dist/workspaceDiagnostics');

const error = (path, message, line = 1) => ({ path, severity: 'Error', line, column: 4, code: 'TS1234', source: 'typescript', message });

test('formats bounded diagnostics with file and source coordinates', () => {
	const result = formatWorkspaceDiagnostics('src/app.ts', [error('src/app.ts', 'Cannot find name x')]);
	assert.match(result, /Diagnostics for src\/app\.ts \(1\)/);
	assert.match(result, /FILE src\/app\.ts\n- Error L1:4 \[TS1234\] \(typescript\): Cannot find name x/);
});

test('returns an explicit empty result and caps diagnostic count', () => {
	assert.equal(formatWorkspaceDiagnostics('workspace', []), 'No diagnostics found for workspace.');
	const result = formatWorkspaceDiagnostics('workspace', Array.from({ length: 3 }, (_, index) => error(`src/${index}.ts`, `error ${index}`)), 2);
	assert.match(result, /\(2, more omitted\)/);
	assert.equal((result.match(/^FILE /gm) || []).length, 2);
});

test('sanitizes diagnostic text and enforces the output size limit', () => {
	const sanitized = formatWorkspaceDiagnostics('workspace', [error('src/app.ts', 'first line\nsecond\u0000 line')]);
	assert.match(sanitized, /first line second line/);
	assert.equal(sanitized.includes('\u0000'), false);
	const bounded = formatWorkspaceDiagnostics('workspace', [error('src/app.ts', 'x'.repeat(200))], 100, 64);
	assert.equal(bounded.length, 64);
	assert.match(bounded, /\[diagnostics truncated\]$/);
});

test('drops malformed diagnostic coordinates and paths', () => {
	const result = formatWorkspaceDiagnostics('workspace', [
		error('src/app.ts', 'valid'),
		{ ...error('../outside.ts', 'unsafe'), path: '../outside.ts' },
		{ ...error('src/bad.ts', 'invalid line'), line: 0 },
		{ ...error('src/control.ts', 'invalid path'), path: 'src/\nbad.ts' }
	]);
	assert.equal((result.match(/^FILE /gm) || []).length, 1);
	assert.equal(result.includes('outside.ts'), false);
});
