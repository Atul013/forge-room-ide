const assert = require('node:assert/strict');
const test = require('node:test');
const { unifiedReviewDiff } = require('../dist/reviewDiff.js');

test('creates a unified diff for a changed file', () => {
	const diff = unifiedReviewDiff([{ path: 'src/app.ts', before: 'one\ntwo\n', after: 'one\nthree\n' }]);
	assert.match(diff, /^diff --git a\/src\/app\.ts b\/src\/app\.ts/m);
	assert.match(diff, /^@@ -1,2 \+1,2 @@$/m);
	assert.match(diff, /^-two$/m);
	assert.match(diff, /^\+three$/m);
	assert.doesNotMatch(diff, /No newline at end of file/);
});

test('marks a created file and a missing final newline', () => {
	const diff = unifiedReviewDiff([{ path: 'new file.md', after: 'hello', created: true }]);
	assert.match(diff, /^new file mode 100644$/m);
	assert.match(diff, /^--- \/dev\/null$/m);
	assert.match(diff, /^@@ -0,0 \+1,1 @@$/m);
	assert.match(diff, /^\\ No newline at end of file$/m);
});

test('represents replacing a non-empty file with an empty file', () => {
	const diff = unifiedReviewDiff([{ path: 'empty.txt', before: 'value', after: '' }]);
	assert.match(diff, /^@@ -1,1 \+0,0 @@$/m);
	assert.match(diff, /^-value$/m);
	assert.doesNotMatch(diff, /^(?!\+\+\+)\+/m);
});

test('normalizes line endings and escapes path control characters', () => {
	const diff = unifiedReviewDiff([{ path: 'folder\nname.ts', before: 'old\r\n', after: 'new\r\n' }]);
	assert.match(diff, /a\/folder%0Aname\.ts b\/folder%0Aname\.ts/);
	assert.match(diff, /^-old$/m);
	assert.match(diff, /^\+new$/m);
	assert.doesNotMatch(diff, /\r/);
});
