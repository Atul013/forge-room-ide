const test = require('node:test');
const assert = require('node:assert/strict');
const { prepareExactTextEdit, workspaceEditCheckpoint } = require('../dist/workspaceEdit');
const { findRollbackConflict } = require('../dist/rollbackGuard');

test('prepares exactly one unique text replacement without mutating the source', () => {
	const source = 'first\nconst value = 1;\nlast';
	assert.deepEqual(prepareExactTextEdit(source, 'const value = 1;', 'const value = 2;'), { text: 'first\nconst value = 2;\nlast', start: 6, end: 22 });
	assert.equal(source, 'first\nconst value = 1;\nlast');
});

test('rejects missing, duplicate, empty, unchanged, and oversized replacements', () => {
	assert.equal(prepareExactTextEdit('a', 'missing', 'x'), undefined);
	assert.equal(prepareExactTextEdit('x x', 'x', 'y'), undefined);
	assert.equal(prepareExactTextEdit('a', '', 'x'), undefined);
	assert.equal(prepareExactTextEdit('a', 'a', 'a'), undefined);
	assert.equal(prepareExactTextEdit('ab', 'a', 'x', 1), undefined);
	assert.equal(prepareExactTextEdit('a', 'a'.repeat(3), 'x', 10, 2), undefined);
});

test('approved exact edits create ordinary single-file rollback checkpoints', () => {
	const checkpoint = workspaceEditCheckpoint({ id: 'checkpoint-1', root: 'file:///project', createdAt: '2026-10-21T00:00:00.000Z', path: 'src/app.ts', before: 'const value = 1;', after: 'const value = 2;', task: { id: 'task-1', title: 'Update value' } });
	assert.deepEqual(checkpoint.files, [{ path: 'src/app.ts', before: 'const value = 1;', after: 'const value = 2;' }]);
	assert.deepEqual(checkpoint.task, { id: 'task-1', title: 'Update value' });
	assert.equal(findRollbackConflict(checkpoint.files, () => ({ exists: true, insideWorkspace: true, content: 'const value = 2;' })), undefined);
	assert.deepEqual(findRollbackConflict(checkpoint.files, () => ({ exists: true, insideWorkspace: true, content: 'const value = 2;\n// later edit' })), { path: 'src/app.ts', reason: 'changed' });
});
