const test = require('node:test');
const assert = require('node:assert/strict');
const { acpSessionStorageKey, normalizeStoredAcpSession } = require('../dist/acpSessionState');

test('ACP session keys stay stable for one workspace, mode, command and argument list', () => {
	const key = acpSessionStorageKey('file:///workspace', 'agent', ['--mode', 'safe'], 'Ask');
	assert.match(key, /^teamLobby\.acpSession\.v1\.[a-f0-9]{64}$/);
	assert.equal(key, acpSessionStorageKey('file:///workspace', 'agent', ['--mode', 'safe'], 'Ask'));
	assert.notEqual(key, acpSessionStorageKey('file:///other', 'agent', ['--mode', 'safe'], 'Ask'));
	assert.notEqual(key, acpSessionStorageKey('file:///workspace', 'agent', ['--mode', 'safe'], 'Plan'));
	assert.notEqual(key, acpSessionStorageKey('file:///workspace', 'agent', ['--mode', 'other'], 'Ask'));
});

test('ACP session state rejects malformed IDs, auth methods and timestamps', () => {
	assert.deepEqual(normalizeStoredAcpSession({ sessionId: 'opaque-session', authMethodId: 'account-a', updatedAt: 10 }), { sessionId: 'opaque-session', authMethodId: 'account-a', updatedAt: 10 });
	for (const value of [undefined, [], {}, { sessionId: '\ninvalid', updatedAt: 1 }, { sessionId: 'x'.repeat(501), updatedAt: 1 }, { sessionId: 'ok', authMethodId: '\0bad', updatedAt: 1 }, { sessionId: 'ok', updatedAt: NaN }]) {
		assert.equal(normalizeStoredAcpSession(value), undefined);
	}
});
