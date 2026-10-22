const test = require('node:test');
const assert = require('node:assert/strict');
const { remapLegacySelfTaskOwner, stableParticipantId } = require('../dist/roomIdentity');

test('local member identity resolves to the stable participant ID', () => {
	assert.equal(stableParticipantId('self', 'participant-123'), 'participant-123');
	assert.equal(stableParticipantId('peer-456', 'participant-123'), 'peer-456');
	assert.equal(stableParticipantId('self', undefined), 'self');
});

test('legacy task owner placeholders migrate without changing unrelated task data', () => {
	const oldTask = { id: 'task-1', ownerId: 'self', owner: 'Sam', status: 'open' };
	assert.deepEqual(remapLegacySelfTaskOwner(oldTask, 'participant-123'), { ...oldTask, ownerId: 'participant-123' });
	assert.equal(remapLegacySelfTaskOwner(oldTask, undefined), oldTask);
	const peerTask = { ...oldTask, ownerId: 'peer-456' };
	assert.equal(remapLegacySelfTaskOwner(peerTask, 'participant-123'), peerTask);
});
