const test = require('node:test');
const assert = require('node:assert/strict');
const { formatTaskEstimate, isTaskEstimateMinutes, parseTaskEstimateInput } = require('../dist/taskEstimate');

test('estimate parser accepts bounded whole minutes and blank optional values', () => {
	assert.deepEqual(parseTaskEstimateInput('90'), { estimateMinutes: 90 });
	assert.deepEqual(parseTaskEstimateInput(''), {});
	assert.equal(parseTaskEstimateInput('0').error !== undefined, true);
	assert.equal(parseTaskEstimateInput('10081').error !== undefined, true);
	assert.equal(parseTaskEstimateInput('1.5').error !== undefined, true);
	assert.equal(isTaskEstimateMinutes(60), true);
	assert.equal(isTaskEstimateMinutes(Number.MAX_SAFE_INTEGER), false);
});

test('estimate formatter keeps minutes readable at minute, hour and day scales', () => {
	assert.equal(formatTaskEstimate(30), '30m');
	assert.equal(formatTaskEstimate(90), '1h 30m');
	assert.equal(formatTaskEstimate(1440), '1d');
	assert.equal(formatTaskEstimate(undefined), 'estimate unknown');
});
