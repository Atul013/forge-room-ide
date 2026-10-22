const assert = require('node:assert/strict');
const test = require('node:test');
const { isAcceptanceCriteria, parseAcceptanceCriteria } = require('../dist/taskCriteria.js');

test('parses and trims optional pipe-separated task criteria', () => {
	assert.deepEqual(parseAcceptanceCriteria('  shows an error  |  saves the result '), { criteria: ['shows an error', 'saves the result'] });
	assert.deepEqual(parseAcceptanceCriteria(''), { criteria: [] });
});

test('rejects too many, oversized, and malformed criteria', () => {
	assert.ok(parseAcceptanceCriteria(Array(9).fill('done').join('|')).error);
	assert.ok(parseAcceptanceCriteria('x'.repeat(181)).error);
	assert.ok(parseAcceptanceCriteria(Array(8).fill('x'.repeat(150)).join('|')).error);
	assert.equal(isAcceptanceCriteria(['done', 'works']), true);
	assert.equal(isAcceptanceCriteria(['']), false);
	assert.equal(isAcceptanceCriteria(Array(9).fill('done')), false);
	assert.ok(parseAcceptanceCriteria('same check|same check').error);
	assert.equal(isAcceptanceCriteria(['legacy duplicate', 'legacy duplicate']), true);
});
