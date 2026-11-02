const assert = require('node:assert/strict');
const test = require('node:test');
const { mergeCommitActivity } = require('../dist/gitEvents.js');

test('marks a commit with multiple parents as a merge activity', () => {
	assert.equal(mergeCommitActivity({ hash: '1234567890abcdef', message: 'Merge feature branch\n\nDetails', parents: ['main', 'feature'] }), '12345678 Merge feature branch');
});

test('does not label ordinary or fast-forward commits as merges', () => {
	assert.equal(mergeCommitActivity({ hash: 'abcdef123456', message: 'Regular commit', parents: ['parent'] }), undefined);
	assert.equal(mergeCommitActivity({ hash: 'abcdef123456', message: 'Missing parent metadata' }), undefined);
});
