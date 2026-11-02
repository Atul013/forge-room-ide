const test = require('node:test');
const assert = require('node:assert/strict');
const { formatRoomActivitySummary } = require('../dist/roomActivitySummary');

test('activity summary exports task status and safe event metadata only', () => {
	const result = formatRoomActivitySummary('Forge Demo', '2026-10-25T12:00:00.000Z', [
		{ title: 'API', owner: 'Kim', status: 'done', priority: 'high' },
		{ title: 'Demo', owner: '', status: 'inProgress' }
	], [
		{ id: 'a', at: '2026-10-25T11:00:00Z', kind: 'test', name: 'Kim' },
		{ id: 'b', at: '2026-10-25T11:30:00Z', kind: 'message', name: 'Lee', detail: 'private chat secret' },
		{ id: 'c', at: '2026-10-25T11:45:00Z', kind: 'review', name: 'Lee', detail: 'sensitive review body', patch: 'UNIQUE_PATCH_PAYLOAD' },
		{ id: 'd', at: '2026-10-25T11:50:00Z', kind: 'handoff', name: 'Kim', recipient: 'private', detail: 'direct message' },
		{ id: 'e', at: '2026-10-25T11:55:00Z', kind: 'decision', name: 'Kim', detail: 'Decision: shared API; rationale excluded' }
	]);
	assert.match(result, /2 total; 1 done, 1 in progress, 0 open/);
	assert.match(result, /Kim — test/);
	assert.match(result, /Lee — review/);
	assert.ok(result.includes('decision'));
	assert.doesNotMatch(result, /private chat secret|sensitive review body|UNIQUE_PATCH_PAYLOAD|rationale excluded/);
	assert.match(result, /excludes direct messages, message and note text, code, file paths, Git patches/);
});

test('activity summary orders and caps events and task list', () => {
	const events = Array.from({ length: 35 }, (_, index) => ({ id: String(index).padStart(2, '0'), at: new Date(Date.UTC(2026, 0, 1, 0, index)).toISOString(), kind: 'task', name: `Member ${index}` }));
	const tasks = Array.from({ length: 60 }, (_, index) => ({ title: `Task ${index}`, owner: 'Owner', status: 'open' }));
	const result = formatRoomActivitySummary('', 'now', tasks, events);
	assert.match(result, /maximum 30/);
	assert.match(result, /Member 34/);
	assert.doesNotMatch(result, /Member 4 — task/);
	assert.match(result, /Task 49/);
	assert.match(result, /Room tasks \(50 of 60\)/);
	assert.doesNotMatch(result, /Task 50/);
});

test('activity summary escapes controls and excludes invalid or unknown activity categories', () => {
	const result = formatRoomActivitySummary('Demo\n[Room]', '2026-10-25T12:00:00.000Z', [], [
		{ id: 'bad-date', at: 'not a date', kind: 'task', name: 'A' },
		{ id: 'private', at: '2026-10-25T11:00:00Z', kind: 'message', name: 'B' },
		{ id: 'path', at: '2026-10-25T11:00:00Z', kind: 'edit', name: 'C' }
	]);
	assert.match(result, /Demo \\\[Room\\\]/);
	assert.match(result, /No eligible room activity recorded/);
});
