const test = require('node:test');
const assert = require('node:assert/strict');
const { formatWorkspaceSymbols } = require('../dist/workspaceSymbols');

const symbol = (name, path = 'src/app.ts', line = 3) => ({ path, name, kind: 'Function', containerName: 'Controller', line });

test('formats symbol names, kinds, containers and file locations without source content', () => {
	const result = formatWorkspaceSymbols('createTask', [symbol('createTask')]);
	assert.match(result, /Symbols matching "createTask" \(1\)/);
	assert.match(result, /FILE src\/app\.ts\n- Function createTask in Controller:3/);
	assert.equal(result.includes('source text'), false);
});

test('reports empty results and caps result count', () => {
	assert.equal(formatWorkspaceSymbols('Missing', []), 'No symbols found for "Missing" in the selected workspace.');
	const result = formatWorkspaceSymbols('task', [symbol('taskA'), symbol('taskB', 'src/b.ts'), symbol('taskC', 'src/c.ts')], 2);
	assert.match(result, /\(2, more omitted\)/);
	assert.equal((result.match(/^FILE /gm) || []).length, 2);
});

test('sanitizes symbol labels and bounds output length', () => {
	const sanitized = formatWorkspaceSymbols('query', [symbol('Bad\nName\u0000')]);
	assert.match(sanitized, /Bad Name/);
	assert.equal(sanitized.includes('\u0000'), false);
	const bounded = formatWorkspaceSymbols('q', [symbol('x'.repeat(200))], 100, 60);
	assert.equal(bounded.length, 60);
	assert.match(bounded, /\[symbols truncated\]$/);
});

test('drops unsafe paths and invalid locations', () => {
	const result = formatWorkspaceSymbols('q', [
		symbol('valid'),
		symbol('escape', '../outside.ts'),
		symbol('absolute', 'C:\\outside.ts'),
		symbol('bad-line', 'src/bad.ts', 0),
		symbol('bad-control', 'src/\nfile.ts')
	]);
	assert.equal((result.match(/^FILE /gm) || []).length, 1);
	assert.equal(result.includes('outside.ts'), false);
});
