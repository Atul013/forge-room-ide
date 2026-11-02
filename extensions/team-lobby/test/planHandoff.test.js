const assert = require('node:assert/strict');
const test = require('node:test');
const { formatBuildRequestFromPlan } = require('../dist/planHandoff.js');

test('hands the edited plan and original request to Build', () => {
	const request = formatBuildRequestFromPlan('Add room criteria', '1. Add the field\n2. Verify persistence');
	assert.match(request, /Original request: Add room criteria/);
	assert.match(request, /Reviewed implementation plan:\n1\. Add the field\n2\. Verify persistence/);
});
