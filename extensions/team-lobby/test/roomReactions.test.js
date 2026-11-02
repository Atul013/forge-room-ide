const test = require('node:test');
const assert = require('node:assert/strict');
const { isRoomReaction, roomReactionState, validIncomingRoomReaction, validStoredRoomReaction } = require('../dist/roomReactions');

const messageId = 'c70d7fd0-cbe6-4d27-9f64-e399699149f4';
const aliceId = 'f3a61b64-f38a-43e6-9cf7-8406db767954';
const bobId = '0a71548e-c104-4a2a-8d9a-6a856e0f6a80';

test('room reactions accept only the supported emoji set', () => {
	assert.equal(isRoomReaction('👍'), true);
	assert.equal(isRoomReaction('💥'), false);
	assert.equal(isRoomReaction('👍<script>'), false);
	assert.equal(validIncomingRoomReaction({ kind: 'reaction', detail: '❤️', reaction: '❤️', reactionTo: messageId }), true);
	assert.equal(validIncomingRoomReaction({ kind: 'reaction', detail: 'secret', reaction: '❤️', reactionTo: messageId }), false);
	assert.equal(validIncomingRoomReaction({ kind: 'reaction', detail: '❤️', reaction: '❤️', reactionTo: '../message' }), false);
	assert.equal(validStoredRoomReaction({ kind: 'reaction', detail: '👍', reaction: '👍', reactionTo: messageId, reactionById: aliceId, reactionActive: true }), true);
	assert.equal(validStoredRoomReaction({ kind: 'reaction', detail: '👍', reaction: '👍', reactionTo: messageId, reactionById: aliceId, reactionActive: true, recipientId: bobId }), false);
});

test('reaction state toggles per participant and emoji, without changing other reactions', () => {
	const events = [
		{ kind: 'reaction', reactionTo: messageId, reaction: '👍', reactionById: aliceId, reactionActive: true },
		{ kind: 'reaction', reactionTo: messageId, reaction: '👍', reactionById: bobId, reactionActive: true },
		{ kind: 'reaction', reactionTo: messageId, reaction: '❤️', reactionById: aliceId, reactionActive: true },
		{ kind: 'reaction', reactionTo: messageId, reaction: '👍', reactionById: aliceId, reactionActive: false },
		{ kind: 'reaction', reactionTo: 'other-message', reaction: '😂', reactionById: bobId, reactionActive: true }
	];
	const state = roomReactionState(events, messageId, aliceId);
	assert.deepEqual(state.find(item => item.reaction === '👍'), { reaction: '👍', count: 1, active: false });
	assert.deepEqual(state.find(item => item.reaction === '❤️'), { reaction: '❤️', count: 1, active: true });
	assert.deepEqual(state.find(item => item.reaction === '😂'), { reaction: '😂', count: 0, active: false });
});
