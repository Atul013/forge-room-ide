const test = require('node:test');
const assert = require('node:assert/strict');
const { searchRoomMessages, searchVisibleRoomMessages } = require('../dist/roomMessageSearch');

const messages = Array.from({ length: 60 }, (_, index) => ({ kind: 'message', id: String(index), name: index % 2 ? 'Ada' : 'Lin', detail: `Build update ${index}`, at: new Date(index * 1000).toISOString() }));

test('message archive search matches message text and participant, newest first', () => {
	const page = searchRoomMessages(messages, 'ada build');
	assert.equal(page.total, 30);
	assert.equal(page.results.length, 25);
	assert.equal(page.results[0].id, '59');
	assert.equal(page.results.at(-1).id, '11');
});

test('message archive search paginates exact totals and bounds page size', () => {
	const first = searchRoomMessages(messages, 'build', 0, 25);
	const second = searchRoomMessages(messages, 'build', 25, 100);
	assert.equal(first.total, 60);
	assert.equal(first.results.length, 25);
	assert.equal(second.results.length, 35);
	assert.deepEqual(searchRoomMessages(messages, 'absent').results, []);
});

test('participant message search excludes direct messages outside that participant boundary', () => {
	const archive = [
		{ kind: 'message', id: 'public', name: 'Ada', detail: 'release decision' },
		{ kind: 'message', id: 'private', name: 'Ada', detail: 'release secret', senderId: 'ada-id', recipientId: 'lin-id', recipient: 'Lin' },
		{ kind: 'message', id: 'invalid-private', name: 'Ada', detail: 'release legacy', recipient: 'Lin' }
	];
	assert.deepEqual(searchVisibleRoomMessages(archive, 'guest-id', 'release').results.map(event => event.id), ['public']);
	assert.deepEqual(searchVisibleRoomMessages(archive, 'lin-id', 'release').results.map(event => event.id), ['private', 'public']);
});

test('room history search finds explicit decisions as well as messages', () => {
	const history = [
		{ kind: 'message', id: 'chat', name: 'Ada', detail: 'We should consider the shared API.' },
		{ kind: 'decision', id: 'decision', name: 'Lin', detail: 'Decision: Use the shared API\nRationale: Keep auth consistent.' },
		{ kind: 'task', id: 'task', name: 'Ada', detail: 'Use the shared API' }
	];
	assert.deepEqual(searchRoomMessages(history, 'shared API').results.map(event => event.id), ['decision', 'chat']);
});

