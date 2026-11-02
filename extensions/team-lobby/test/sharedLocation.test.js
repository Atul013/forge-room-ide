const test = require('node:test');
const assert = require('node:assert/strict');
const { locationUpdateAllowed, prepareEditorLocationUpdate, validSharedLocation } = require('../dist/sharedLocation');

test('shared editor location accepts a safe relative file and bounded one-based line', () => {
	assert.equal(validSharedLocation('src/main.ts', 1), true);
	assert.equal(validSharedLocation('src\\nested\\main.ts', 42), true);
	assert.equal(validSharedLocation('src/main.ts', 10_000_000), true);
});

test('live location stream is rate limited to four updates per second', () => {
	assert.equal(locationUpdateAllowed(undefined, 1_000), true);
	assert.equal(locationUpdateAllowed(1_000, 1_249), false);
	assert.equal(locationUpdateAllowed(1_000, 1_250), true);
	assert.equal(locationUpdateAllowed(1_000, Number.NaN), false);
});

test('live location wire messages bind actor identity and contain only path and line', () => {
	const actor = { id: '12345678-1234-1234-1234-123456789abc', name: 'Ada', role: 'human' };
	const accepted = prepareEditorLocationUpdate({ active: true, file: 'src/main.ts', line: 9 }, actor, undefined, 2_000);
	assert.deepEqual(accepted.location, { participantId: actor.id, name: 'Ada', role: 'human', file: 'src/main.ts', line: 9, updatedAt: new Date(2_000).toISOString() });
	assert.equal(Object.hasOwn(accepted.location, 'selectedText'), false);
	assert.equal(prepareEditorLocationUpdate({ active: true, file: '../secret.ts', line: 9 }, actor, undefined, 2_000).location, undefined);
	assert.equal(prepareEditorLocationUpdate({ active: true, file: 'src/main.ts', line: 9, selectedText: 'private source' }, actor, undefined, 2_000).location, undefined);
	assert.deepEqual(prepareEditorLocationUpdate({ active: false }, actor, undefined, 2_000), { cleared: true });
});

test('shared editor location rejects traversal, absolute paths, controls, and invalid line numbers', () => {
	for (const [file, line] of [
		['../outside.ts', 2], ['/etc/passwd', 2], ['C:\\secret.ts', 2], ['src//main.ts', 2], ['src/main.ts', 0], ['src/main.ts', 1.5], ['src/main.ts', Number.MAX_SAFE_INTEGER + 1], ['src/main.ts', '3'], ['src/main.ts', 10_000_001]
	]) { assert.equal(validSharedLocation(file, line), false, `${file} ${line}`); }
});
