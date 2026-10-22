const test = require('node:test');
const assert = require('node:assert/strict');
const { canDeliverMessageEvent, directMessagePeers } = require('../dist/directMessages');

test('direct message routing uses participant IDs rather than duplicate display names', () => {
	const peers = [{ id: 'sender', name: 'Alex' }, { id: 'target', name: 'Alex' }, { id: 'other', name: 'Alex' }];
	assert.deepEqual(directMessagePeers(peers, 'target', 'sender').map(peer => peer.id), ['sender', 'target']);
});

test('direct message history is limited to sender and recipient; legacy name-only messages are withheld', () => {
	const event = { kind: 'message', recipient: 'Alex', recipientId: 'target', senderId: 'sender' };
	assert.equal(canDeliverMessageEvent(event, 'sender'), true);
	assert.equal(canDeliverMessageEvent(event, 'target'), true);
	assert.equal(canDeliverMessageEvent(event, 'other'), false);
	assert.equal(canDeliverMessageEvent({ kind: 'message', recipient: 'Alex' }, 'other'), false);
	assert.equal(canDeliverMessageEvent({ kind: 'post' }, 'other'), true);
});
