const test = require('node:test');
const assert = require('node:assert/strict');
const { findRollbackConflict } = require('../dist/rollbackGuard');

const files = [
	{ path: 'src/agent.ts', after: 'Build version' },
	{ path: 'src/new.ts', after: 'new file', created: true }
];
const snapshot = content => ({ exists: true, insideWorkspace: true, createdFileIsSymlink: false, content });

test('rollback proceeds only when every target still matches the applied checkpoint', () => {
	assert.equal(findRollbackConflict(files, file => snapshot(file.after)), undefined);
});

test('rollback refuses the whole checkpoint when a later edit would be overwritten', () => {
	const conflict = findRollbackConflict(files, file => snapshot(file.path === 'src/agent.ts' ? 'Build version plus unrelated edit' : file.after));
	assert.deepEqual(conflict, { path: 'src/agent.ts', reason: 'changed' });
});

test('rollback refuses missing, escaped, and symlinked created-file targets', () => {
	assert.deepEqual(findRollbackConflict(files, file => ({ ...snapshot(file.after), exists: file.path !== 'src/agent.ts' })), { path: 'src/agent.ts', reason: 'missing' });
	assert.deepEqual(findRollbackConflict(files, file => ({ ...snapshot(file.after), insideWorkspace: file.path !== 'src/agent.ts' })), { path: 'src/agent.ts', reason: 'outside-workspace' });
	assert.deepEqual(findRollbackConflict(files, file => ({ ...snapshot(file.after), createdFileIsSymlink: file.created })), { path: 'src/new.ts', reason: 'unsafe-created-file' });
});
