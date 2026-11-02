const test = require('node:test');
const assert = require('node:assert/strict');
const { formatRoomDecision, isValidRoomDecisionDetail, maxDecisionTitleLength, maxDecisionRationaleLength } = require('../dist/roomDecision');

test('formats a bounded, searchable room decision with its rationale', () => {
	assert.equal(formatRoomDecision('Use the shared API', 'Avoid duplicated account rules.'), 'Decision: Use the shared API\nRationale: Avoid duplicated account rules.');
	assert.equal(formatRoomDecision('   ', 'Reason'), undefined);
	assert.equal(formatRoomDecision('Decision', '  '), undefined);
	assert.equal(formatRoomDecision('x'.repeat(maxDecisionTitleLength + 1), 'Reason'), undefined);
	assert.equal(formatRoomDecision('Decision', 'x'.repeat(maxDecisionRationaleLength + 1)), undefined);
	assert.equal(formatRoomDecision('Title\nInjected', 'Reason'), undefined);
	assert.equal(isValidRoomDecisionDetail('Decision: Pick A\nRationale: It fits.'), true);
	assert.equal(isValidRoomDecisionDetail('Unstructured guest text'), false);
	assert.equal(isValidRoomDecisionDetail('Decision: Pick A\nRationale:  '), false);
});
