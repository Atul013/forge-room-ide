const test = require('node:test');
const assert = require('node:assert/strict');
const { commentAnchorLabel, validCommentAnchor } = require('../dist/commentAnchor');

test('accepts a bounded one-based comment line range', () => {
	assert.equal(validCommentAnchor({ startLine: 8, endLine: 12 }), true);
	assert.equal(commentAnchorLabel({ startLine: 8, endLine: 12 }), 'lines 8–12');
	assert.equal(commentAnchorLabel({ startLine: 8, endLine: 8 }), 'line 8');
});

test('rejects invalid, reversed, fractional, or oversized anchors', () => {
	for (const value of [null, {}, { startLine: 0, endLine: 1 }, { startLine: 3, endLine: 2 }, { startLine: 1.5, endLine: 2 }, { startLine: 1, endLine: 201 }, { startLine: 10_000_001, endLine: 10_000_001 }]) {
		assert.equal(validCommentAnchor(value), false);
	}
});
