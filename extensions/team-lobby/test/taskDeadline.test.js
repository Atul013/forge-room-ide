const test = require('node:test');
const assert = require('node:assert/strict');
const { deadlineAfterDuration, defaultHackathonDurationMs, isStoredTaskDeadline, parseFutureDeadlineInput, parseTaskDeadlineInput } = require('../dist/taskDeadline');

test('normalizes explicit-timezone task deadlines to UTC', () => {
	assert.deepEqual(parseTaskDeadlineInput('2026-10-22T18:00+05:30'), { dueAt: '2026-10-22T12:30:00.000Z' });
	assert.deepEqual(parseTaskDeadlineInput('2026-10-22T12:30:00Z'), { dueAt: '2026-10-22T12:30:00.000Z' });
	assert.deepEqual(parseTaskDeadlineInput(''), {});
});

test('rejects timezone-free, invalid calendar, clock, and offset inputs', () => {
	for (const value of ['2026-10-22T12:30', '2026-02-30T12:30Z', '2026-10-22T25:30Z', '2026-10-22T12:30+25:00', '2026-10-22']) {
		assert.ok(parseTaskDeadlineInput(value).error, value);
	}
});

test('accepts only normalized UTC timestamps from room clients', () => {
	assert.equal(isStoredTaskDeadline('2026-10-22T12:30:00.000Z'), true);
	assert.equal(isStoredTaskDeadline('2026-10-22T12:30+05:30'), false);
	assert.equal(isStoredTaskDeadline('2026-02-30T12:30:00.000Z'), false);
});

test('hackathon countdown accepts future deadlines, rejects past ones, and allows clearing', () => {
	const now = Date.parse('2026-10-21T00:00:00.000Z');
	assert.deepEqual(parseFutureDeadlineInput('2026-10-22T12:00Z', now), { dueAt: '2026-10-22T12:00:00.000Z' });
	assert.match(parseFutureDeadlineInput('2026-10-20T12:00Z', now).error, /future/);
	assert.deepEqual(parseFutureDeadlineInput('', now), {});
});

test('24-hour hackathon preset uses a fixed duration from the host start time', () => {
	const now = Date.parse('2026-10-21T10:15:00.000Z');
	assert.equal(deadlineAfterDuration(defaultHackathonDurationMs, now), '2026-10-22T10:15:00.000Z');
	assert.equal(deadlineAfterDuration(0, now), undefined);
	assert.equal(deadlineAfterDuration(Number.MAX_SAFE_INTEGER, now), undefined);
});
