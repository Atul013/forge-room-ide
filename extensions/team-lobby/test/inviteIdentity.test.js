const assert = require('node:assert/strict');
const test = require('node:test');
const { acceptInviteIdentity } = require('../dist/inviteIdentity.js');

test('pins the first name and role to a participant ID for an invite', () => {
	const participants = new Map();
	assert.deepEqual(acceptInviteIdentity(participants, 'participant-1', { name: '  Ada  ', role: 'human' }), { accepted: true, identity: { name: 'Ada', role: 'human' } });
	assert.deepEqual(acceptInviteIdentity(participants, 'participant-1', { name: 'Ada', role: 'human' }), { accepted: true, identity: { name: 'Ada', role: 'human' } });
	assert.deepEqual(acceptInviteIdentity(participants, 'participant-1', { name: 'Mallory', role: 'human' }), { accepted: false });
	assert.deepEqual(acceptInviteIdentity(participants, 'participant-1', { name: 'Ada', role: 'agent' }), { accepted: false });
});
