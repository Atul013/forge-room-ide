const assert = require('node:assert/strict');
const test = require('node:test');
const { updateReportedConflictPaths } = require('../dist/conflictPaths.js');

test('reports each connected opt-in conflict path once until it resolves', () => {
	const reported = new Set();
	assert.deepEqual(updateReportedConflictPaths(reported, ['src/a.ts'], true, true), ['src/a.ts']);
	assert.deepEqual(updateReportedConflictPaths(reported, ['src/a.ts'], true, true), []);
	assert.deepEqual(updateReportedConflictPaths(reported, [], true, true), []);
	assert.deepEqual(updateReportedConflictPaths(reported, ['src/a.ts'], true, true), ['src/a.ts']);
});

test('clears suppression when sharing is disabled or the room disconnects', () => {
	const reported = new Set(['src/a.ts']);
	assert.deepEqual(updateReportedConflictPaths(reported, ['src/a.ts'], false, true), []);
	assert.equal(reported.size, 0);
	updateReportedConflictPaths(reported, ['src/a.ts'], true, true);
	assert.deepEqual(updateReportedConflictPaths(reported, ['src/a.ts'], true, false), []);
	assert.equal(reported.size, 0);
});

test('ignores empty paths and caps each state update at one hundred paths', () => {
	const reported = new Set();
	const paths = Array.from({ length: 101 }, (_, index) => `src/${index}.ts`);
	const added = updateReportedConflictPaths(reported, ['', ...paths], true, true);
	assert.equal(added.length, 100);
	assert.equal(reported.size, 100);
});
