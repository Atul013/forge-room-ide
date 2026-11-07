const test = require('node:test');
const assert = require('node:assert/strict');
const { isRoomMilestones, milestoneStatus, parseRoomMilestonesInput } = require('../dist/roomMilestones');

const now = Date.parse('2026-10-25T12:00:00.000Z');

test('milestone input parses, normalizes and sorts future timezone-aware entries', () => {
	const result = parseRoomMilestonesInput('Demo | 2026-10-25T16:00Z\nFreeze | 2026-10-25T14:00+00:00', now);
	assert.deepEqual(result.milestones.map(item => item.label), ['Freeze', 'Demo']);
	assert.equal(result.milestones[0].dueAt, '2026-10-25T14:00:00.000Z');
});

test('milestone input rejects malformed, duplicate, past and excessive entries', () => {
	assert.match(parseRoomMilestonesInput('Demo 2026-10-25T16:00Z', now).error, /one milestone per line/);
	assert.match(parseRoomMilestonesInput('Demo | 2026-10-25T16:00\nDemo | 2026-10-25T17:00Z', now).error, /timezone/);
	assert.match(parseRoomMilestonesInput('Demo | 2026-10-25T16:00Z\nDemo | 2026-10-25T17:00Z', now).error, /unique/);
	assert.match(parseRoomMilestonesInput('Demo | 2026-10-25T11:00Z', now).error, /future/);
	assert.match(parseRoomMilestonesInput(Array.from({ length: 21 }, (_, i) => `M${i} | 2026-10-26T12:00Z`).join('\n'), now).error, /no more than 20/);
});

test('stored milestones validate IDs, labels, dates, and uniqueness', () => {
	const entries = [{ id: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa', label: 'Demo', dueAt: '2026-10-25T13:00:00.000Z' }];
	assert.equal(isRoomMilestones(entries), true);
	assert.equal(isRoomMilestones([...entries, ...entries]), false);
	assert.equal(isRoomMilestones([{ ...entries[0], id: 'bad' }]), false);
	assert.equal(isRoomMilestones([{ ...entries[0], label: 'Demo\nSubmit' }]), false);
});

test('milestone status distinguishes passed, within-one-hour, and upcoming', () => {
	assert.equal(milestoneStatus({ id: 'a', label: 'a', dueAt: '2026-10-25T11:59:00.000Z' }, now), 'passed');
	assert.equal(milestoneStatus({ id: 'b', label: 'b', dueAt: '2026-10-25T13:00:00.000Z' }, now), 'soon');
	assert.equal(milestoneStatus({ id: 'c', label: 'c', dueAt: '2026-10-25T14:00:00.000Z' }, now), 'upcoming');
});
