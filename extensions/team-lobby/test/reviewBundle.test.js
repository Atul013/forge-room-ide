const test = require('node:test');
const assert = require('node:assert/strict');
const { formatReviewRisks } = require('../dist/reviewBundle');

test('review risks are labeled as user-entered and kept inside quoted Markdown', () => {
	assert.equal(formatReviewRisks('Known issue\n# heading'), '> User-entered reviewer notes; not independently verified.\n> Known issue\n> # heading');
});

test('empty risks are stated explicitly', () => {
	assert.equal(formatReviewRisks(' \r\n\t'), '> No additional risks recorded.');
});

test('review risk notes remove control characters and are capped', () => {
	const output = formatReviewRisks('A\u0000B' + 'x'.repeat(2_500));
	assert.equal(output.includes('\u0000'), false);
	assert.ok(output.length <= 2_100);
	assert.match(output, /^> User-entered reviewer notes/);
});
