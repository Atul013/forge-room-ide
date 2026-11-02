const test = require('node:test');
const assert = require('node:assert/strict');
const { validRoomArtifact } = require('../dist/roomArtifacts');

test('room diffs carry only a bounded reviewed patch and relative file path', () => {
	assert.equal(validRoomArtifact({ kind: 'diff', detail: 'Reviewed Git patch', file: 'src/main.ts', patch: '@@ -1 +1 @@\n-old\n+new' }), true);
	assert.equal(validRoomArtifact({ kind: 'diff', detail: 'Reviewed Git patch', file: 'src/main.ts', patch: 'x'.repeat(8_001) }), false);
	assert.equal(validRoomArtifact({ kind: 'diff', detail: 'Reviewed Git patch', file: '../secret.txt', patch: '@@ patch' }), false);
});

test('anchored comments share a line range and comment text without selected source', () => {
	assert.equal(validRoomArtifact({ kind: 'comment', detail: 'Please check this edge case', file: 'src/main.ts', anchor: { startLine: 8, endLine: 9 } }), true);
	for (const field of ['content', 'prompt', 'selectedText', 'sourceText', 'terminalOutput']) {
		assert.equal(validRoomArtifact({ kind: 'comment', detail: 'Review this', file: 'src/main.ts', anchor: { startLine: 8, endLine: 9 }, [field]: 'private source' }), false, field);
	}
	assert.equal(validRoomArtifact({ kind: 'comment', detail: 'Review this', file: 'src/main.ts' }), false);
	assert.equal(validRoomArtifact({ kind: 'comment', detail: 'Review this', file: 'src/main.ts', anchor: { startLine: 0, endLine: 9 } }), false);
});

