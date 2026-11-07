const test = require('node:test');
const assert = require('node:assert/strict');
const { threadMessages, threadReplyDenial } = require('../dist/roomThreads');

const senderId = 'participant-a';
const recipientId = 'participant-b';
const outsiderId = 'participant-c';

test('room-thread replies remain public and require an existing message', () => {
	const events = [{ id: 'root', kind: 'message' }, { id: 'reply', kind: 'message', replyToId: 'root' }];
	assert.equal(threadReplyDenial('root', events, senderId), undefined);
	assert.equal(threadReplyDenial('root', events, senderId, recipientId), 'Replies to room messages must stay visible in the room.');
	assert.equal(threadReplyDenial('missing', events, senderId), 'The message this reply references is no longer available.');
	assert.deepEqual(threadMessages(events, 'root').map(event => event.id), ['root', 'reply']);
	assert.deepEqual(threadMessages(events, 'missing'), []);
});

test('direct-message thread replies stay within original two participants', () => {
	const events = [{ id: 'root', kind: 'message', senderId, recipientId }];
	assert.equal(threadReplyDenial('root', events, senderId, recipientId), undefined);
	assert.equal(threadReplyDenial('root', events, recipientId, senderId), undefined);
	assert.equal(threadReplyDenial('root', events, outsiderId, recipientId), 'Only participants in the original direct-message conversation can reply.');
	assert.equal(threadReplyDenial('root', events, senderId, outsiderId), 'A direct-message reply must stay within the original conversation.');
	assert.equal(threadReplyDenial('reply', [...events, { id: 'reply', kind: 'message', senderId, recipientId, replyToId: 'root' }], outsiderId, recipientId), 'Only participants in the original direct-message conversation can reply.');
});
