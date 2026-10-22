const test = require('node:test');
const assert = require('node:assert/strict');
const { roomHistoryForExport } = require('../dist/roomHistoryExport');

test('room history export excludes direct messages by default and preserves room messages', () => {
	const events = [
		{ id: 'public', kind: 'message', detail: 'room update' },
		{ id: 'private', kind: 'message', recipientId: 'peer-b', senderId: 'peer-a', detail: 'private update' },
		{ id: 'other', kind: 'task', detail: 'task changed' }
	];
	assert.deepEqual(roomHistoryForExport(events, false).map(event => event.id), ['public', 'other']);
	assert.deepEqual(roomHistoryForExport(events, true).map(event => event.id), ['public', 'private', 'other']);
	assert.deepEqual(events.map(event => event.id), ['public', 'private', 'other']);
});
