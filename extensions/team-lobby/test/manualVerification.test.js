const test = require('node:test');
const assert = require('node:assert/strict');
const { manualVerificationActivityDetail } = require('../dist/manualVerification');

test('manual verification events state that outcomes are participant reported', () => {
	assert.match(manualVerificationActivityDetail('Room task', 'Tests', 'passed'), /^manually reported verification for task .* Tests passed \(exit 0\)\./);
	assert.match(manualVerificationActivityDetail('Room task', 'Build', 'failed'), /failed \(exit 1\)/);
	assert.match(manualVerificationActivityDetail(undefined, 'Lint', 'stopped'), /unlinked room check.*completed without a process exit code/);
});

test('manual verification labels reject control characters, overlong and credential-like names', () => {
	assert.equal(manualVerificationActivityDetail('Task', 'Tests\nextra', 'passed'), undefined);
	assert.equal(manualVerificationActivityDetail('Task', 'x'.repeat(81), 'passed'), undefined);
	assert.equal(manualVerificationActivityDetail('Task', 'API_TOKEN check', 'passed'), undefined);
});

test('manual verification details bound and flatten remote task titles', () => {
	const detail = manualVerificationActivityDetail('Task\n'.repeat(100), 'Tests', 'passed');
	assert.ok(detail.length <= 500);
	assert.doesNotMatch(detail, /\n/);
});
