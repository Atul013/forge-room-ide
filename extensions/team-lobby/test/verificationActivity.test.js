const test = require('node:test');
const assert = require('node:assert/strict');
const { canShareVerificationCommand, commandForRoom, redactVerificationCommand, verificationActivityDetail, verificationCommandEvidence } = require('../dist/verificationActivity');

test('verification command sharing requires one bounded single-line command', () => {
	assert.equal(canShareVerificationCommand('npm test'), true);
	assert.equal(canShareVerificationCommand('  '), false);
	assert.equal(canShareVerificationCommand('npm test\npowershell'), false);
	assert.equal(canShareVerificationCommand('x'.repeat(221)), false);
});

test('verification room activity includes only explicitly supplied command details and remains bounded', () => {
	assert.match(verificationActivityDetail('Room task', 'Unit tests', 0), /passed \(exit 0\)\.$/);
	assert.match(verificationActivityDetail('Room task', 'Unit tests', 0, 'npm test'), /Command: npm test/);
	assert.ok(verificationActivityDetail('x'.repeat(600), 'Unit tests', 0, 'npm test').length <= 500);
});

test('verification commands redact common credential values before room sharing', () => {
	assert.equal(redactVerificationCommand('npm test --token=abc123'), 'npm test --token=[REDACTED]');
	assert.equal(redactVerificationCommand('API_TOKEN=abc123 npm test'), 'API_TOKEN=[REDACTED] npm test');
	assert.equal(redactVerificationCommand('curl -H "Authorization: Bearer abc123" https://a:b@example.test'), 'curl -H "Authorization: Bearer [REDACTED]" https://[REDACTED]@example.test');
	assert.equal(redactVerificationCommand('echo AKIA1234567890ABCDEF github_pat_123456789012345678901234'), 'echo [REDACTED_TOKEN] [REDACTED_TOKEN]');
	assert.equal(redactVerificationCommand('curl -u user:pass http://a:b@example.test'), 'curl -u [REDACTED] http://[REDACTED]@example.test');
	assert.doesNotMatch(verificationActivityDetail('Task', 'Check', 0, 'npm test --password=secret123'), /secret123/);
});

test('verification evidence keeps a redacted local command while room sharing stays opt-in', () => {
	const local = verificationCommandEvidence('npm test --token=secret123', false);
	assert.deepEqual(local, { command: 'npm test --token=[REDACTED]' });
	assert.equal(commandForRoom(local), undefined);
	const shared = verificationCommandEvidence('npm test --token=secret123', true);
	assert.deepEqual(shared, { command: 'npm test --token=[REDACTED]', sharedWithRoom: true });
	assert.equal(commandForRoom(shared), 'npm test --token=[REDACTED]');
	assert.deepEqual(verificationCommandEvidence(undefined, true), {});
	assert.deepEqual(verificationCommandEvidence('x'.repeat(221), false), {});
	assert.deepEqual(verificationCommandEvidence('npm test\nrm -rf .', true), {});
});
